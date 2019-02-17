#[macro_use]
extern crate nom;
#[macro_use]
extern crate bencher;

extern crate fnv;

use fnv::FnvHashMap as HashMap;
use bencher::{Bencher, black_box};

use nom::{digit, be_u32, IResult, Err, ErrorKind, InputTakeAtPosition, Convert, recognize_float,
  ParseTo, Slice, InputLength, Needed,HexDisplay};


named!(first<u32>, flat_map!(digit, parse_to!(u32)));
named!(second<u32>, call!(be_u32));

pub fn or<'b, I: Clone, O, E>(input: I, fns: &'b[&'b Fn(I) -> IResult<I, O, E>]) -> IResult<I, O, E> {
  let mut index = 0;

  for f in fns.iter() {
    match f(input.clone()) {
      Err(Err::Error(_)) => {},
      rest => return rest,
    }

  }

  Err(Err::Error(error_position!(input, ErrorKind::Alt)))
}

pub fn separated<I: Clone, O1, O2, O3, E, F, G, H>(input: I, first: F, sep: G, second: H) -> IResult<I, (O1, O3), E>
  where F: Fn(I) -> IResult<I, O1, E>,
        G: Fn(I) -> IResult<I, O2, E>,
        H: Fn(I) -> IResult<I, O3, E> {

  let (input, o1) = first(input)?;
  let (input, _)  = sep(input)?;
  second(input).map(|(i, o2)| (i, (o1, o2)))
}

pub fn delimited<I: Clone, O1, O2, O3, E, F, G, H>(input: I, first: F, sep: G, second: H) -> IResult<I, O2, E>
  where F: Fn(I) -> IResult<I, O1, E>,
        G: Fn(I) -> IResult<I, O2, E>,
        H: Fn(I) -> IResult<I, O3, E> {

  let (input, _) = first(input)?;
  let (input, o2)  = sep(input)?;
  second(input).map(|(i, _)| (i, o2))
}

pub fn take_while<'a, T: 'a, F>(input: &'a [T], cond: F) -> IResult<&'a [T], &'a [T]>
  where F: Fn(T) -> bool,
        &'a [T]: nom::InputTakeAtPosition<Item=T> {
  input.split_at_position(|c| !cond(c))
}

pub fn take_while1<'a, T: 'a, F>(input: &'a [T], cond: F) -> IResult<&'a [T], &'a [T]>
  where F: Fn(T) -> bool,
        &'a [T]: nom::InputTakeAtPosition<Item=T> {
  input.split_at_position1(|c| !cond(c), ErrorKind::TakeWhile1)
}

pub fn map<I, O1, O2, F, G>(input: I, first: F, second: G) -> IResult<I, O2>
  where F: Fn(I) -> IResult<I, O1>,
        G: Fn(O1) -> O2 {

  first(input).map(|(i, o1)| (i, second(o1)))
}

pub fn flat_map<I: Clone+From<O1>, O1, O2, F, G>(input: I, first: F, second: G) -> IResult<I, O2>
  where F: Fn(I) -> IResult<I, O1>,
        G: Fn(O1) -> IResult<O1, O2> {

  let (i, o1) = first(input)?;
  second(o1).map(|(_, o2)| (i, o2)).map_err(Err::convert)
}

pub fn many0<I: Clone+InputLength, O, F>(input: I, mut f: F) -> IResult<I, Vec<O>>
  where F: FnMut(I) -> IResult<I, O> {

  let mut i = input;
  let mut acc = Vec::with_capacity(4);

  loop {
    let i_ = i.clone();
    match f(i_) {
      Err(_) => return Ok((i, acc)),
      Ok((i2, o)) => {
        if i.input_len() == i2.input_len() {
          return Err(Err::Error(error_position!(i, ErrorKind::Many0)))
        }

        i = i2;
        acc.push(o);

        if i.input_len() == 0 {
          return Ok((i, acc));
        }
      }
    }
  }
}

pub fn many1<I: Clone+InputLength, O, F>(input: I, mut f: F) -> IResult<I, Vec<O>>
  where F: FnMut(I) -> IResult<I, O> {

  let mut i = input;
  let mut acc = Vec::with_capacity(4);

  loop {
    let i_ = i.clone();
    match f(i_) {
      Err(_) => if acc.is_empty() {
        return Err(Err::Error(error_position!(i, ErrorKind::Many1)))
      } else {
        return Ok((i, acc));
      },
      Ok((i2, o)) => {
        if i.input_len() == i2.input_len() {
          return Err(Err::Error(error_position!(i, ErrorKind::Many1)))
        }

        i = i2;
        acc.push(o);

        if i.input_len() == 0 {
          if acc.is_empty() {
            return Err(Err::Error(error_position!(i, ErrorKind::Many1)))
          } else {
            return Ok((i, acc));
          }
        }
      }
    }
  }
}

pub fn separated_list<I: Clone+InputLength, O, O2, F, G>(input: I, mut sep: G, mut f: F) -> IResult<I, Vec<O>>
  where F: FnMut(I) -> IResult<I, O>,
        G: FnMut(I) -> IResult<I, O2> {
  let mut acc = Vec::new();
  let (input, o) = f(input)?;
  acc.push(o);

  let mut i = input;

  loop {
    if i.input_len() == 0 {
      return Ok((i, acc));
    }

    let i_ = i.clone();
    match sep(i_) {
      Err(_) => return Ok((i, acc)),
      Ok((i2, _)) => {
        if i.input_len() == i2.input_len() {
          return Err(Err::Error(error_position!(i, ErrorKind::Many0)))
        }

        let i2_ = i2.clone();
        match f(i2_) {
          Err(_) => return Ok((i, acc)),
          Ok((i3, o)) => {
            if i2.input_len() == i3.input_len() {
              return Err(Err::Error(error_position!(i, ErrorKind::Many0)))
            }

            i = i3;
            acc.push(o);
          }
        }
      }
    }
  }
}

pub fn char(c: char) -> impl Fn(&[u8]) -> IResult<&[u8], char> {

  move |i:&[u8]| {
    if i.len() == 0 {
      Err(Err::Incomplete(Needed::Unknown))
    } else {
      //beware of utf8
      if i[0] as char == c {
        Ok((&i[1..], c))
      } else {
        Err(Err::Error(error_position!(i, ErrorKind::Char)))
      }
    }
  }
}

pub fn tag<'b, 'a: 'b>(t: &'a [u8]) -> impl Fn(&'b [u8]) -> IResult<&'b [u8], &'b [u8]> {
  move |i:&'b [u8]| {
    tag!(i, t)
  }
}

pub fn value<I, O1, O2, F>(input: I, f: F, o: O2) -> IResult<I, O2>
  where F: Fn(I) -> IResult<I, O1> {

  f(input).map(|(i, _)| (i, o))
}

/****************************/

fn parser(input: &[u8]) -> IResult<&[u8], u32> {
  or(input, &[&first, &second])
}

fn test_many(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
  let mut counter = 0;
  let res = many0(input,
    |i| {
      counter = counter + 1;
      tag!(i, "abcd")
    });

  println!("counter: {}", counter);
  res
}

#[test]
fn manytest() {
  test_many(&b"abcdabcdabcd"[..]);
  panic!();
}

