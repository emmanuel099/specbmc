extern crate falcon;
#[macro_use]
extern crate error_chain;
extern crate num_bigint;
extern crate num_traits;

pub mod translator;
pub mod ir;

pub mod error {
    error_chain! {
        types {
            Error, ErrorKind, ResultExt, Result;
        }

        foreign_links {
            Falcon(::falcon::error::Error);
            ParseBigIntError(::num_bigint::ParseBigIntError);
            NullError(::std::ffi::NulError);
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
