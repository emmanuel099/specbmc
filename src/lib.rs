#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate derive_builder;

pub mod cex;
pub mod environment;
pub mod expr;
pub mod hir;
pub mod ir;
pub mod lir;
pub mod loader;
pub mod mir;
pub mod solver;
pub mod util;

pub mod error {
    error_chain! {
        types {
            Error, ErrorKind, ResultExt, Result;
        }

        foreign_links {
            Falcon(::falcon::error::Error);
            ParseBigIntError(::num_bigint::ParseBigIntError);
            RSmt2(::rsmt2::errors::Error);
            IOError(::std::io::Error);
            NullError(::std::ffi::NulError);
            SerdeYAML(::serde_yaml::Error);
        }

        errors {
            Analysis(m: String) {
                description("An error in the analysis")
                display("Analysis error: {}", m)
            }
            Sort {
                description("Sort error")
                display("Sort error, bits differ incorrectly")
            }
        }
    }
}
