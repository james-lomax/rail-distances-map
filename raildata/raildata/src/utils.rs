/** Copyright James Lomax 2020 */

use std::io;

/** Appends context to an error if the result is an err */
pub fn append_err_context<T>(r: io::Result<T>, context: String) -> io::Result<T> {
    match r {
        Ok(x) => Ok(x),
        Err(e) => {
            let kind = e.kind();
            let err = match e.into_inner() {
                Some(inner_err) => format!("{}: {}", context, inner_err),
                None => context
            };
            Err(io::Error::new(kind, err))
        }
    }
}
