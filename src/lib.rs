// Copyright 2021 Adam Greig
// Licensed under the Apache-2.0 and MIT licenses.

//! spidap
//!
//! SPI flash access using CMSIS-DAP probes in JTAG mode.

use spi_flash::FlashAccess;
use jtagdap::dap::{DAP, Error as DAPError};
use jtagdap::jtag::{Sequences, Error as JTAGError};
use jtagdap::bitvec::{bytes_to_bits, bits_to_bytes, Error as BitvecError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("DAP error")]
    DAP(#[from] DAPError),
    #[error("JTAG error")]
    JTAG(#[from] JTAGError),
    #[error("Bitvec error")]
    Bitvec(#[from] BitvecError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct SPIFlash {
    dap: DAP,
}

impl SPIFlash {
    pub fn new(dap: DAP) -> Self {
        SPIFlash { dap }
    }

    pub fn release(self) -> DAP {
        self.dap
    }

    /// Create a new Sequences object for running custom JTAG sequences.
    fn sequences(&self) -> Sequences {
        Sequences::with_dap(&self.dap)
    }
}

impl FlashAccess for SPIFlash {
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        // Bit-reverse each byte to flip JTAG's LSb-first to SPI's MSb-first.
        let data: Vec<u8> = data.iter().map(|x| x.reverse_bits()).collect();
        let bits = bytes_to_bits(&data, data.len() * 8)?;
        self.sequences()
            .write(&bits, false)?
            .mode(&[true])?
            .run()?;
        Ok(())
    }

    fn exchange(&mut self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let data: Vec<u8> = data.iter().map(|x| x.reverse_bits()).collect();
        let bits = bytes_to_bits(&data, data.len() * 8)?;
        let result = self.sequences()
                         .exchange(&bits, false)?
                         .mode(&[true])?
                         .run()?;
        let result = bits_to_bytes(&result);
        let result: Vec<u8> = result.iter().map(|x| x.reverse_bits()).collect();
        Ok(result)
    }
}
