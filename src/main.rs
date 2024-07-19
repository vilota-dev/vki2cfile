use std::time::Duration;
use std::{fs::File, io::Read, path::PathBuf};
use std::process::abort;
use clap::{Args, Parser, Subcommand};
use i2cdev::core::I2CDevice;
use i2cdev::{core::{I2CMessage, I2CTransfer}, linux::LinuxI2CDevice};
use serde::{Deserialize, Serialize};

/// Total size of the EEPROM in bytes.
const EEPROM_SIZE: u16 = 8192;
/// Offset to the address of the first byte in EEPROM where the metadata resides.
const METADATA_OFFSET: u16 = 0;
/// Offset to the address of the first byte in EEPROM where the content resides.
const CONTENT_OFFSET: u16 = 32;
/// Maximum size of content that can be stored in the EEPROM memory.
const MAX_CONTENT_SIZE: u16 = EEPROM_SIZE - CONTENT_OFFSET;

/// CRC algorithm used.
const CRC: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_USB);

/// Sanity check.
static _METDATA_SIZE_ASSERTION: () = assert!(std::mem::size_of::<Metadata>() <= CONTENT_OFFSET as usize);

/// Metadata stored in the memory
/// 
/// Note: If you modify this structure, take care to ensure backwards compatiblity.
#[repr(C)]
#[derive(Serialize, Deserialize)]
struct Metadata {
    unused: [u8; 28],
    content_crc: u16,
    content_size: u16,
}


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Command {
    #[command(subcommand)]
    subcommand: Sub
}

#[derive(Subcommand)]
enum Sub {
    Read(ReadCommand),
    Write(WriteCommand),
}

/// Read a file from EEPROM into the filesystem.
#[derive(Args)]
struct ReadCommand {
    /// Read the file out regardless whether CRC validation succeeds or not. 
    #[arg(long)]
    ignore_crc: bool,

    /// Read the file out even if it is empty (i.e. zero-sized).
    #[arg(long)]
    allow_empty: bool,

    /// Path in the filesystem to write the file into.
    destination: PathBuf
}

/// Write a file from the filesystem into EEPROM.
#[derive(Args)]
struct WriteCommand {
    /// Path in the filesystem to read the file from.
    source: PathBuf
}

fn open_device() -> LinuxI2CDevice {
    const DEVICE_PATH: &str = "/dev/i2c-3";
    const EEPROM_ADDRESS: u16 = 0x50;

    match LinuxI2CDevice::new(DEVICE_PATH, EEPROM_ADDRESS) {
        Ok(device) => device,
        Err(error) => {
            eprintln!("Failed to open device: {error}");
            abort()
        },
    }
}


fn main() {
    match Command::parse().subcommand {
        Sub::Read(read) => {
            let mut device = open_device();
            let mut metadata_buffer = vec![0; std::mem::size_of::<Metadata>()];

            if let Err(error) = device.transfer(&mut [
                I2CMessage::write(&METADATA_OFFSET.to_be_bytes()),
                I2CMessage::read(metadata_buffer.as_mut_slice()),
            ]) {
                eprintln!("Failed to read file metadata from EEPROM: {error}.");
                abort()
            }
            
            std::thread::sleep(Duration::from_millis(10));

            let Ok(metadata) = bincode::deserialize::<Metadata>(metadata_buffer.as_slice()) else {
                eprintln!("Invalid file metadata in EEPROM.");
                abort()
            };

            if metadata.content_size > MAX_CONTENT_SIZE {
                eprintln!("Invalid file size in EEPROM: exceeds maximum possible ({} > {}).", metadata.content_size, MAX_CONTENT_SIZE);
                abort()
            }

            if !read.allow_empty && metadata.content_size == 0 {
                eprintln!("File in EEPROM is empty or does not exists.");
                abort()
            }

            let mut content_buffer = vec![0; metadata.content_size as usize];

            if let Err(error) = device.transfer(&mut [
                I2CMessage::write(&CONTENT_OFFSET.to_be_bytes()),
                I2CMessage::read(content_buffer.as_mut_slice()),
            ]) {
                eprintln!("Failed to read file contents from EEPROM: {error}.");
                abort()
            }

            if !read.ignore_crc {
                let crc = CRC.checksum(&content_buffer.as_slice());
    
                if crc != metadata.content_crc {
                    eprintln!("File does not exist or is corrupted: CRC of file content does not match CRC in its metadata.");
                    abort()
                }
            }

            if let Err(error) = std::fs::write(read.destination.as_path(), content_buffer.as_slice()) {
                eprintln!("Failed to write to file '{:?}': {error}", read.destination);
                abort()
            }
        }
        Sub::Write(write) => {
            let mut device = open_device();
            let mut content_buffer = Vec::default();
            let mut metadata_buffer = Vec::from(METADATA_OFFSET.to_be_bytes());

            let file_size = match File::open(write.source.as_path()).and_then(|mut f| f.read_to_end(&mut content_buffer)) {
                Ok(file_size) => file_size,
                Err(error) => {
                    eprintln!("Failed to read from file '{:?}': {error}", write.source);
                    abort()
                }
            };

            if file_size > MAX_CONTENT_SIZE as usize {
                eprintln!("File '{:?}' is too large. Max allowable size is {MAX_CONTENT_SIZE} bytes.", write.source);
                abort()
            }

            let metadata = Metadata {
                unused: Default::default(),
                content_crc: CRC.checksum(content_buffer.as_slice()),
                content_size: file_size as u16,
            };

            // Unwrap should always succeed.
            bincode::serialize_into(&mut metadata_buffer, &metadata).unwrap();

            // Sanity check that the serialized size is the same as the struct size.
            if metadata_buffer.len() - 2 != std::mem::size_of::<Metadata>() {
                eprintln!("Internal error: unexpected metadata size.");
                abort()
            }

            // Write file metadata.
            if let Err(error) = device.write(metadata_buffer.as_slice()) {
                eprintln!("Failed to write file metadata into EEPROM: {error}.");
                abort()
            }

            std::thread::sleep(Duration::from_millis(10));

            // Write file content.
            let mut buffer = vec![0_u8; 34];

            for (index, chunk) in content_buffer.chunks(32).enumerate() {
                let offset = CONTENT_OFFSET + 32 * (index as u16);
                let size = 2 + chunk.len();

                buffer[0..2].copy_from_slice(&offset.to_be_bytes());
                buffer[2..size].copy_from_slice(chunk);

                if let Err(error) = device.write(&buffer[0..size]) {
                    eprintln!("Failed to write file into EEPROM: {error}.");
                    abort()
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}