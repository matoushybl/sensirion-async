#![cfg_attr(not(test), no_std)]

pub mod scd30;
pub mod scd4x;
pub mod sgp40;
pub mod sht4x;
pub mod sps30;
mod vocalg;

use crc_all::Crc;
use embedded_hal_async::{delay::DelayNs, i2c::I2c};

pub trait SensirionCommand {
    fn raw(&self) -> u16;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ParsingError {
    Crc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<Inner: core::fmt::Debug> {
    Bus(Inner),
    Parsing(ParsingError),
}

impl<E: embedded_hal_async::i2c::Error> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Self::Bus(e)
    }
}

impl<T: core::fmt::Debug> From<ParsingError> for Error<T> {
    fn from(e: ParsingError) -> Self {
        Error::Parsing(e)
    }
}

pub struct SensirionI2c<T>
where
    T: I2c,
{
    bus: T,
    crc: SensirionCrc,
}

impl<'a, T> SensirionI2c<T>
where
    T: I2c,
{
    pub fn new(bus: T) -> SensirionI2c<T> {
        Self {
            bus,
            crc: Default::default(),
        }
    }

    pub async fn read_word<Command: SensirionCommand>(
        &mut self,
        address: u8,
        command: Command,
        check_crc: bool,
    ) -> Result<u16, Error<T::Error>> {
        self.bus
            .write(address, &command.raw().to_be_bytes())
            .await?;

        let mut buffer = [0; 3];
        self.bus.read(address, &mut buffer).await?;

        if check_crc {
            let crc = self.crc.calculate(&buffer[..2]);
            if crc != buffer[2] {
                return Err(Error::Parsing(ParsingError::Crc));
            }
        }

        Ok(u16::from_be_bytes(buffer[..2].try_into().unwrap()))
    }

    pub async fn write_word<Command: SensirionCommand>(
        &mut self,
        address: u8,
        command: Command,
        word: u16,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; 5];

        buffer[0..2].copy_from_slice(&command.raw().to_be_bytes());
        buffer[2..4].copy_from_slice(&word.to_be_bytes());

        buffer[4] = self.crc.calculate(&buffer[2..4]);

        self.bus.write(address, &buffer).await?;

        Ok(())
    }

    pub async fn write_command(
        &mut self,
        address: u8,
        command: impl SensirionCommand,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; 3];

        buffer[0..2].copy_from_slice(&command.raw().to_be_bytes());
        buffer[2] = self.crc.calculate(&buffer[0..2]);

        self.bus.write(address, &buffer).await?;

        Ok(())
    }

    pub async fn write_read_raw(
        &mut self,
        address: u8,
        write_buffer: &[u8],
        read_buffer: &mut [u8],
        delay_ms: u32,
        delay: &mut impl DelayNs,
    ) -> Result<(), Error<T::Error>> {
        self.bus.write(address, write_buffer).await?;

        delay.delay_ms(delay_ms).await;

        self.bus.read(address, read_buffer).await?;
        Ok(())
    }

    pub async fn read_raw<Command: SensirionCommand>(
        &mut self,
        address: u8,
        command: Command,
        buffer: &mut [u8],
    ) -> Result<(), Error<T::Error>> {
        self.bus
            .write(address, &command.raw().to_be_bytes())
            .await?;

        self.bus.read(address, buffer).await?;

        Ok(())
    }
}

pub(crate) struct SensirionCrc {
    inner: Crc<u8>,
}

impl SensirionCrc {
    pub fn new() -> Self {
        Self {
            inner: crc_all::Crc::<u8>::new(0x31, 8, 0xff, 0x00, false),
        }
    }

    pub fn calculate(&mut self, input: &[u8]) -> u8 {
        self.inner.init();
        self.inner.update(input);
        self.inner.finish()
    }
}

impl Default for SensirionCrc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc() {
        let mut crc = SensirionCrc::new();

        assert_eq!(0x92, crc.calculate(&[0xbe, 0xef]));
    }
}
