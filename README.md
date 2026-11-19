# Introduction
Simple tool to read and write a single file to the EEPROM of the MK24C64 device.


# Build
Run the command `cargo build --release` in the project root directory.

Afterwards, the binary can be found at `./target/release`.


# Run
Simply execute the produced binary `sudo ./vki2cfile <COMMAND> <FILE>` where *COMMAND* is "read" or "write" and
*FILE* is the source or desination file.

For more information on the arguments and additional options, simply run `sudo ./vki2cfile --help`. 

Note that root permission is needed for this tool.

# Note
Run without root permission:
- `sudo apt install i2c-tools`
- `pi ALL=(ALL) NOPASSWD: /homr/pi/vki2cfle/target/release/vki2cfile`
-  `sudo usermod -aG i2c $USER`
- Reboot
# Pinhole factory use

The storage format is unchanged: 28 reserved bytes followed by content CRC and
length, then up to 8160 bytes of content at offset 32. Pinhole identity is stored
in the content JSON, not in reserved bytes. The application layer is
`vk_flash_calibration pinhole clear/dump/flash` in `vk-system`.

Writes reject empty/oversize files and verify metadata and content by reading
back from EEPROM. A failed verification returns a nonzero status. This is not
an atomic-write or recovery mechanism; explicitly reinitialize unreadable devices.

Install the executable on the capture machine's `PATH`, for example with
`cargo install --path . --locked`. Run `cargo test --offline` for the storage-layout
regression test. Actual I2C writes require hardware validation.
