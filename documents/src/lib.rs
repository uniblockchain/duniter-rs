//  Copyright (C) 2018  The Duniter Project Developers.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Implements the Duniter Documents Protocol.

#![cfg_attr(feature = "strict", deny(warnings))]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces
)]

extern crate base58;
extern crate base64;
extern crate byteorder;
extern crate crypto;
extern crate duniter_crypto;
extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod blockstamp;
mod currencies_codes;
pub mod v10;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use currencies_codes::*;
use duniter_crypto::hashs::Hash;
use duniter_crypto::keys::*;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Error, Formatter};
use std::io::Cursor;
use std::mem;

pub use blockstamp::{Blockstamp, PreviousBlockstamp};

#[derive(Parser)]
#[grammar = "documents_grammar.pest"]
/// Parser for Documents
struct DocumentsParser;

/// List of blockchain protocol versions.
#[derive(Debug, Clone)]
pub enum BlockchainProtocol {
    /// Version 10.
    V10(Box<v10::V10Document>),
    /// Version 11. (not done yet, but defined for tests)
    V11(),
}

/// Currency name
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize, Hash)]
pub struct CurrencyName(pub String);

impl Default for CurrencyName {
    fn default() -> CurrencyName {
        CurrencyName(String::from("default_currency"))
    }
}

impl Display for CurrencyName {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

/// CurrencyCodeError
#[derive(Debug)]
pub enum CurrencyCodeError {
    /// UnknowCurrencyCode
    UnknowCurrencyCode(),
    /// IoError
    IoError(::std::io::Error),
    /// UnknowCurrencyName
    UnknowCurrencyName(),
}

impl From<::std::io::Error> for CurrencyCodeError {
    fn from(error: ::std::io::Error) -> Self {
        CurrencyCodeError::IoError(error)
    }
}

impl CurrencyName {
    /// Convert bytes to CurrencyName
    pub fn from(currency_code: [u8; 2]) -> Result<Self, CurrencyCodeError> {
        let mut currency_code_bytes = Cursor::new(currency_code.to_vec());
        let currency_code = currency_code_bytes.read_u16::<BigEndian>()?;
        Self::from_u16(currency_code)
    }
    /// Convert u16 to CurrencyName
    pub fn from_u16(currency_code: u16) -> Result<Self, CurrencyCodeError> {
        match currency_code {
            tmp if tmp == *CURRENCY_NULL => Ok(CurrencyName(String::from(""))),
            tmp if tmp == *CURRENCY_G1 => Ok(CurrencyName(String::from("g1"))),
            tmp if tmp == *CURRENCY_G1_TEST => Ok(CurrencyName(String::from("g1-test"))),
            _ => Err(CurrencyCodeError::UnknowCurrencyCode()),
        }
    }
    /// Convert CurrencyName to bytes
    pub fn to_bytes(&self) -> Result<[u8; 2], CurrencyCodeError> {
        let currency_code = match self.0.as_str() {
            "g1" => *CURRENCY_G1,
            "g1-test" => *CURRENCY_G1_TEST,
            _ => return Err(CurrencyCodeError::UnknowCurrencyName()),
        };
        let mut buffer = [0u8; mem::size_of::<u16>()];
        buffer
            .as_mut()
            .write_u16::<BigEndian>(currency_code)
            .expect("Unable to write");
        Ok(buffer)
    }
}

/// A block Id.
#[derive(Copy, Clone, Debug, Deserialize, Ord, PartialEq, PartialOrd, Eq, Hash, Serialize)]
pub struct BlockId(pub u32);

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

/// Wrapper of a block hash.
#[derive(Copy, Clone, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize)]
pub struct BlockHash(pub Hash);

impl Display for BlockHash {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.0.to_hex())
    }
}

impl Debug for BlockHash {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "BlockHash({})", self)
    }
}

/// trait providing commun methods for any documents of any protocol version.
///
/// # Design choice
///
/// Allow only ed25519 for protocol 10 and many differents
/// schemes for protocol 11 through a proxy type.
pub trait Document: Debug + Clone {
    /// Type of the `PublicKey` used by the document.
    type PublicKey: PublicKey;
    /// Data type of the currency code used by the document.
    type CurrencyType: ?Sized;

    /// Get document version.
    fn version(&self) -> u16;

    /// Get document currency.
    fn currency(&self) -> &Self::CurrencyType;

    /// Get document blockstamp
    fn blockstamp(&self) -> Blockstamp;

    /// Iterate over document issuers.
    fn issuers(&self) -> &Vec<Self::PublicKey>;

    /// Iterate over document signatures.
    fn signatures(&self) -> &Vec<<Self::PublicKey as PublicKey>::Signature>;

    /// Get document as bytes for signature verification.
    fn as_bytes(&self) -> &[u8];

    /// Verify signatures of document content (as text format)
    fn verify_signatures(&self) -> VerificationResult {
        let issuers_count = self.issuers().len();
        let signatures_count = self.signatures().len();

        if issuers_count != signatures_count {
            VerificationResult::IncompletePairs(issuers_count, signatures_count)
        } else {
            let issuers = self.issuers();
            let signatures = self.signatures();
            let mismatches: Vec<_> = issuers
                .iter()
                .zip(signatures)
                .enumerate()
                .filter(|&(_, (key, signature))| !key.verify(self.as_bytes(), signature))
                .map(|(i, _)| i)
                .collect();

            if mismatches.is_empty() {
                VerificationResult::Valid()
            } else {
                VerificationResult::Invalid(mismatches)
            }
        }
    }
}

/// List of possible results for signature verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// All signatures are valid.
    Valid(),
    /// Not same amount of issuers and signatures.
    /// (issuers count, signatures count)
    IncompletePairs(usize, usize),
    /// Signatures don't match.
    /// List of mismatching pairs indexes.
    Invalid(Vec<usize>),
}

/// Trait allowing access to the document through it's proper protocol version.
///
/// This trait is generic over `P` providing all supported protocol version variants.
///
/// A lifetime is specified to allow enum variants to hold references to the document.
pub trait IntoSpecializedDocument<P> {
    /// Get a protocol-specific document wrapped in an enum variant.
    fn into_specialized(self) -> P;
}

/// Trait helper for building new documents.
pub trait DocumentBuilder {
    /// Type of the builded document.
    type Document: Document;

    /// Type of the private keys signing the documents.
    type PrivateKey: PrivateKey<
        Signature = <<Self::Document as Document>::PublicKey as PublicKey>::Signature,
    >;

    /// Build a document with provided signatures.
    fn build_with_signature(
        &self,
        signatures: Vec<<<Self::Document as Document>::PublicKey as PublicKey>::Signature>,
    ) -> Self::Document;

    /// Build a document and sign it with the private key.
    fn build_and_sign(&self, private_keys: Vec<Self::PrivateKey>) -> Self::Document;
}

/// Trait for a document parser from a `S` source
/// format to a `D` document. Will return the
/// parsed document or an `E` error.
pub trait DocumentParser<S, D, E> {
    /// Parse a source and return a document or an error.
    fn parse(source: S) -> Result<D, E>;
}
