#[macro_use]
extern crate nom;

use nom::{digit, be_u32, IResult, Err};


named!(first<u32>, flat_map!(digit, parse_to!(u32)));
named!(second<u32>, call!(be_u32));

/*fn or<'a, 'b, F>(input: &'a [u8], left: F, right: F) -> IResult<&'a [u8], u32>
  where F: &'b Fn(&'a [u8]) -> IResult<&'a [u8], u32> {

    match left(input) {
      Err(Err::Error(_)) => {
        right(input)
      },
      rest => rest
    }
}

fn parser(input: &[u8]) -> IResult<&[u8], u32> {
  or(input, &first, &second)
}
*/

//named!(parser<u32>, alt!(first | second));
fn or<'a, 'b>(input: &'a [u8], fns: &'b[&'b Fn(&'a [u8]) -> IResult<&'a [u8], u32>]) -> IResult<&'a [u8], u32> {
    let left = fns[0];
    let right = fns[1];

    match left(input) {
      Err(Err::Error(_)) => {
        right(input)
      },
      rest => rest
    }
}

fn parser(input: &[u8]) -> IResult<&[u8], u32> {
  or(input, &[&first, &second])
}
