#[macro_use]
extern crate nom;
#[macro_use]
extern crate bencher;

extern crate fnv;

use fnv::FnvHashMap as HashMap;
use bencher::{Bencher, black_box};

use nom::{digit, be_u32, InputTakeAtPosition, Convert, recognize_float,
  ParseTo, Slice, InputLength, HexDisplay, InputTake, Compare, CompareResult, need_more,};

use std::fmt::Debug;

pub type IResult<I, O, E=(I,u32)> = Result<(I, O), Err<E>>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Needed {
  Unknown,
  Size(usize),
}

#[derive(Debug)]
pub enum Err<E> {
  Incomplete(Needed),
  Error(E),
  Failure(E),
}

impl<E1> Err<E1> {
  fn convert<E2: Into<E1>>(other: Err<E2>) -> Self {
    match other {
      Err::Incomplete(i) => Err::Incomplete(i),
      Err::Error(e) => Err::Error(e.into()),
      Err::Failure(e) => Err::Failure(e.into())
    }
  }
}

#[derive(Debug)]
pub enum ErrorKind {
  Alt,
  Many0,
  Many1,
  Char,
  Tag,
  TakeWhile,
  TakeWhile1,
}

pub trait Er<I> {
  fn from_error_kind(input: I, kind: ErrorKind) -> Self;

  fn or(self, other: Self) -> Self;
}

impl<I> Er<I> for (I, u32) {
  fn from_error_kind(input: I, kind: ErrorKind) -> Self {
    (input, 0)
  }

  fn or(self, other: Self) -> Self {
    other
  }
}
#[derive(Debug)]
pub struct Simple<I> {
  i: I,
  e: ErrorKind,
}

impl<I> Er<I> for Simple<I> {
  fn from_error_kind(input: I, kind: ErrorKind) -> Self {
    Simple {
      i: input,
      e: kind,
    }
  }

  fn or(self, other: Self) -> Self {
    other
  }
}

#[derive(Debug)]
enum VerboseKind {
  E(ErrorKind),
  Context(&'static str),
}

#[derive(Debug)]
pub struct Verbose<I> {
  v: Vec<(I, VerboseKind)>,
}

impl<I> Verbose<I> {
  pub fn append(mut self, input: I, context: &'static str) -> Self {
    let k = VerboseKind::Context(context);

    self.v.push((input, k));

    self
  }
}

impl<'a> Er<&'a [u8]> for Verbose<&'a [u8]> {
  fn from_error_kind(input: &'a [u8], kind: ErrorKind) -> Self {
    Verbose {
      v: vec![(input, VerboseKind::E(kind))],
    }
  }

  fn or(self, other: Self) -> Self {
    //println!("or: self: {:?}, other: {:?}", self, other);
    // take the error from the branch that went the farthest
    let p1 = self.v.first().unwrap().0.as_ptr();
    let p2 = other.v.first().unwrap().0.as_ptr();
    //println!("p1: {:x?}, p2: {:x?}", p1, p2);
    if p1 <= p2 {
      other
    } else {
      self
    }
  }
}

pub fn context<I: Clone, O, F>(mut parser: F, s: &'static str) -> impl FnMut(I) -> IResult<I, O, Verbose<I>>
  where F: FnMut(I) -> IResult<I, O, Verbose<I>> {

  move |input: I| {
    match parser(input.clone()) {
      Ok(res) => return Ok(res),
      Err(Err::Incomplete(i)) => Err(Err::Incomplete(i)),
      Err(Err::Error(e)) => Err(Err::Error(e.append(input, s))),
      Err(Err::Failure(e)) => Err(Err::Failure(e.append(input, s))),
    }
  }
}

/*************************/

//named!(first<u32>, flat_map!(digit, parse_to!(u32)));
//named!(second<u32>, call!(be_u32));

pub fn or<'b, I: Clone, O, E: Er<I>>(input: I, fns: &'b[&'b Fn(I) -> IResult<I, O, E>]) -> IResult<I, O, E> {
  let mut index = 0;

  for f in fns.iter() {
    match f(input.clone()) {
      Err(Err::Error(_)) => {},
      rest => return rest,
    }

  }

  Err(Err::Error(E::from_error_kind(input, ErrorKind::Alt)))
}

pub fn separated<I: Clone, O1, O2, O3, E: Er<I>, F, G, H>(input: I, first: F, sep: G, second: H) -> IResult<I, (O1, O3), E>
  where F: Fn(I) -> IResult<I, O1, E>,
        G: Fn(I) -> IResult<I, O2, E>,
        H: Fn(I) -> IResult<I, O3, E> {

  let (input, o1) = first(input)?;
  let (input, _)  = sep(input)?;
  second(input).map(|(i, o2)| (i, (o1, o2)))
}

pub fn delimited<I: Clone, O1, O2, O3, E: Er<I>, F, G, H>(input: I, first: F, sep: G, second: H) -> IResult<I, O2, E>
  where F: Fn(I) -> IResult<I, O1, E>,
        G: Fn(I) -> IResult<I, O2, E>,
        H: Fn(I) -> IResult<I, O3, E> {

  let (input, _) = first(input)?;
  let (input, o2)  = sep(input)?;
  second(input).map(|(i, _)| (i, o2))
}

pub fn take_while<'a, T: 'a, F, E: Er<&'a[T]>>(input: &'a [T], cond: F) -> IResult<&'a [T], &'a [T], E>
  where F: Fn(T) -> bool,
        &'a [T]: nom::InputTakeAtPosition<Item=T> {
  input.split_at_position(|c| !cond(c)).map_err(|_| Err::Error(E::from_error_kind(input, ErrorKind::TakeWhile)))
}

//#[inline(always)]
pub fn take_while1<'a, T: 'a, F, E: Er<&'a[T]>>(input: &'a [T], cond: F) -> IResult<&'a [T], &'a [T], E>
  where F: Fn(T) -> bool,
        &'a [T]: nom::InputTakeAtPosition<Item=T> {
  match input.split_at_position(|c| !cond(c)) {
    Err(_) => Err(Err::Error(E::from_error_kind(input, ErrorKind::TakeWhile1))),
    Ok(s) => if s.1.is_empty() {
      Err(Err::Error(E::from_error_kind(input, ErrorKind::TakeWhile1)))
    } else {
      Ok(s)
    }
  }
}

pub fn map<I, O1, O2, F, G>(input: I, first: F, second: G) -> IResult<I, O2>
  where F: Fn(I) -> IResult<I, O1>,
        G: Fn(O1) -> O2 {

  first(input).map(|(i, o1)| (i, second(o1)))
}

pub fn flat_map<I: Clone+From<O1>, O1, O2, E1: Er<I>+From<E2>, E2: Er<O1>, F, G>(input: I, first: F, second: G) -> IResult<I, O2, E1>
  where F: Fn(I) -> IResult<I, O1, E1>,
        G: Fn(O1) -> IResult<O1, O2, E2> {

  let (i, o1) = first(input)?;
  second(o1).map(|(_, o2)| (i, o2)).map_err(Err::convert)
}

pub fn many0<I: Clone+InputLength, O, E: Er<I>, F>(input: I, mut f: F) -> IResult<I, Vec<O>, E>
  where F: FnMut(I) -> IResult<I, O, E> {

  let mut i = input;
  let mut acc = Vec::with_capacity(4);

  loop {
    let i_ = i.clone();
    match f(i_) {
      Err(_) => return Ok((i, acc)),
      Ok((i2, o)) => {
        if i.input_len() == i2.input_len() {
          return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many0)))
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

pub fn many1<I: Clone+Copy+InputLength, O, E: Er<I>, F>(input: I, f: F) -> IResult<I, Vec<O>, E>
  where F: Fn(I) -> IResult<I, O, E> {
  //many1!(input, f)

  let mut i = input;

  let i_ = i.clone();
  match f(i_) {
    Err(_) => {
      return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many1)))
    },
    Ok((i2, o)) => {
      let mut acc = Vec::with_capacity(4);
      acc.push(o);
      let mut i = i2;

      loop {
        let i_ = i.clone();
        match f(i_) {
          Err(_) => {
            return Ok((i, acc));
          },
          Ok((i2, o)) => {
            if i.input_len() == i2.input_len() {
              return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many1)))
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
  }
}

pub fn separated_list<I: Clone+InputLength, O, O2, E: Er<I>, F, G>(input: I, mut sep: G, mut f: F) -> IResult<I, Vec<O>, E>
  where F: FnMut(I) -> IResult<I, O, E>,
        G: FnMut(I) -> IResult<I, O2, E> {
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
          return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many0)))
        }

        let i2_ = i2.clone();
        match f(i2_) {
          Err(_) => return Ok((i, acc)),
          Ok((i3, o)) => {
            if i2.input_len() == i3.input_len() {
              return Err(Err::Error(E::from_error_kind(i, ErrorKind::Many0)))
            }

            i = i3;
            acc.push(o);
          }
        }
      }
    }
  }
}

//#[inline(always)]
pub fn char<'a, E: Er<&'a[u8]>>(c: char) -> impl Fn(&'a[u8]) -> IResult<&'a[u8], char, E> {

  move |i:&[u8]| {
    if i.len() == 0 {
      Err(Err::Incomplete(Needed::Unknown))
    } else {
      //beware of utf8
      if i[0] as char == c {
        Ok((&i[1..], c))
      } else {
        Err(Err::Error(E::from_error_kind(i, ErrorKind::Char)))
      }
    }
  }
}

pub fn tag<'b, 'a: 'b, E: Er<&'b[u8]>>(t: &'a [u8]) -> impl Fn(&'b [u8]) -> IResult<&'b [u8], &'b [u8], E> {
  move |i:&'b [u8]| {
    let tag_len = t.input_len();
    let res: IResult<_, _, E> = match i.compare(t) {
      CompareResult::Ok => Ok(i.take_split(tag_len)),
      //CompareResult::Incomplete => need_more(i, Needed::Size(tag_len)),
      CompareResult::Incomplete | CompareResult::Error => {
        Err(Err::Error(E::from_error_kind(i, ErrorKind::Tag)))
      }
    };

    res
  }
}

pub fn value<I, O1, O2, E: Er<I>, F>(input: I, f: F, o: O2) -> IResult<I, O2, E>
  where F: Fn(I) -> IResult<I, O1, E> {

  f(input).map(|(i, _)| (i, o))
}

/****************************/

/*
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
*/
