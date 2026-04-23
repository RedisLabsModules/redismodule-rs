use crate::error::Error;
use crate::raw;
use crate::RedisBuffer;

/// Wrapper around RedisModuleIO for type-safe serialization/deserialization.
///
/// This wrapper provides an ergonomic API for working with RedisModuleIO
/// RDB serialization operations while maintaining compatibility with the underlying C bindings.
pub struct RedisModuleIO {
    io: *mut raw::RedisModuleIO,
}

impl RedisModuleIO {
    /// Creates a new RedisModuleIO wrapper from a raw pointer.
    pub fn new(io: *mut raw::RedisModuleIO) -> Self {
        Self { io }
    }

    /// Returns the raw RedisModuleIO pointer.
    pub fn as_ptr(&self) -> *mut raw::RedisModuleIO {
        self.io
    }

    /// Reads a string from the IO stream.
    ///
    /// # Errors
    /// Returns an error if the operation fails or if an IO error occurred.
    pub fn read_string(&mut self) -> Result<String, Error> {
        let string_ptr = raw::load_string(self.io)?;
        Ok(string_ptr.to_string_lossy())
    }

    /// Writes a string to the IO stream.
    pub fn write_string(&mut self, s: &str) {
        raw::save_string(self.io, s);
    }

    /// Reads an unsigned integer from the IO stream.
    ///
    /// # Errors
    /// Returns an error if the operation fails or if an IO error occurred.
    pub fn read_unsigned(&mut self) -> Result<u64, Error> {
        raw::load_unsigned(self.io)
    }

    /// Writes an unsigned integer to the IO stream.
    pub fn write_unsigned(&mut self, val: u64) {
        raw::save_unsigned(self.io, val);
    }

    /// Reads a signed integer from the IO stream.
    ///
    /// # Errors
    /// Returns an error if the operation fails or if an IO error occurred.
    pub fn read_signed(&mut self) -> Result<i64, Error> {
        raw::load_signed(self.io)
    }

    /// Writes a signed integer to the IO stream.
    pub fn write_signed(&mut self, val: i64) {
        raw::save_signed(self.io, val);
    }

    /// Reads a double from the IO stream.
    ///
    /// # Errors
    /// Returns an error if the operation fails or if an IO error occurred.
    pub fn read_double(&mut self) -> Result<f64, Error> {
        raw::load_double(self.io)
    }

    /// Writes a double to the IO stream.
    pub fn write_double(&mut self, val: f64) {
        raw::save_double(self.io, val);
    }

    /// Reads a float from the IO stream.
    ///
    /// # Errors
    /// Returns an error if the operation fails or if an IO error occurred.
    pub fn read_float(&mut self) -> Result<f32, Error> {
        raw::load_float(self.io)
    }

    /// Writes a float to the IO stream.
    pub fn write_float(&mut self, val: f32) {
        raw::save_float(self.io, val);
    }

    /// Reads a string buffer from the IO stream.
    ///
    /// # Errors
    /// Returns an error if the operation fails or if an IO error occurred.
    pub fn read_string_buffer(&mut self) -> Result<RedisBuffer, Error> {
        raw::load_string_buffer(self.io)
    }

    /// Writes a slice to the IO stream.
    pub fn write_slice(&mut self, buf: &[u8]) {
        raw::save_slice(self.io, buf);
    }
}
