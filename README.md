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