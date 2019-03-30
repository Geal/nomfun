#[macro_use]
extern crate nom;
#[macro_use]
extern crate bencher;
extern crate nomfun;

extern crate fnv;

use fnv::FnvHashMap as HashMap;
use bencher::{Bencher, black_box};

use nomfun::*;
use std::fmt::Debug;
use std::str::from_utf8;

pub fn is_string_character(c: u8) -> bool {
  //FIXME: should validate unicode character
  c != b'"' && c != b'\\'
}

pub fn is_space(c: u8) -> bool {
  c == b' ' || c == b'\t' || c == b'\r' || c == b'\n'
}


//named!(sp, take_while!(is_space));
fn sp<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], &'a[u8], E> {
  take_while(input, is_space)
}

fn sp2<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], &'a[u8], E> {
  let chars = b" \t\r\n";

  take_while(input, |c| chars.contains(&c))
}

// compat function because I don't want to rewrite nom::recognize_float just for this
fn convert_rec_float<'a, E: Er<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E> {
  match nom::recognize_float(input) {
    Ok((i, o)) => Ok((i, o)),
    Err(nom::Err::Incomplete(_)) => Err(Err::Incomplete(Needed::Unknown)),
    Err(nom::Err::Error(_)) => Err(Err::Error(E::from_error_kind(input, ErrorKind::ParseTo))),
    Err(nom::Err::Failure(_)) => Err(Err::Failure(E::from_error_kind(input, ErrorKind::ParseTo))),
  }
}

//named!(float<f64>, flat_map!(recognize_float, parse_to!(f64)));
fn float<'a, E: Er<&'a[u8]>>(i: &'a [u8]) -> IResult<&'a [u8], f64, E> {
  //println!("float");
  let second = |i: &'a [u8]| {
    match from_utf8(i).ok().and_then(|s| s.parse::<f64>().ok()) {
      Some(o) => Ok((&i[i.len()..], o)),
      None => Err(Err::Error(E::from_error_kind(i, ErrorKind::ParseTo)))
    }
  };

  flat_map(i, convert_rec_float, second)
}

#[derive(Debug, PartialEq)]
pub enum JsonValue<'a> {
  Str(&'a str),
  Boolean(bool),
  Num(f64),
  Array(Vec<JsonValue<'a>>),
  Object(HashMap<&'a str, JsonValue<'a>>),
}

use std::str;
fn parse_str<'a, E:Er<&'a[u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a str, E> {
  // let's ignore escaping for now
  /*
  //println!("parse_str");
  let res = map_res!(input,
    escaped!(take_while1!(is_string_character), '\\', one_of!("\"bfnrt\\")),
    str::from_utf8
  );
  //println!("parse_str({}) got {:?}", str::from_utf8(input).unwrap(), res);
  res*/
  let (i, data) = take_while(input, |c| c != b'"')?;
  //println!("parse_str: data is {}", from_utf8(data).unwrap());
  match from_utf8(data) {
    Ok(s) => Ok((i, s)),
    Err(_) => Err(Err::Error(E::from_error_kind(input, ErrorKind::ParseTo)))
  }
}

fn string<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], &'a str, E> {
  //println!("string");
  let res = delimited(input, char('\"'), parse_str, char('\"'));
  //println!("string(\"{}\") returned {:?}", str::from_utf8(input).unwrap(), res);
  res
}

fn boolean<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], bool, E> {
  //println!("boolean");
  or(input, &[
   &|i| { value(i, tag(&b"false"[..]), false) },
   &|i| { value(i, tag(&b"true"[..]), true) }
  ])
}

fn array<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], Vec<JsonValue>, E> {
  //println!("array");
  delimited(input,
    char('['),
    |i| separated_list(i, char(','), json_value),
    char(']')
  )
}

fn key_value<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], (&'a str, JsonValue), E> {
  //println!("key_value");
  let res = separated(input, string, char(':'), json_value);
  //println!("key_value(\"{}\") returned {:?}", str::from_utf8(input).unwrap(), res);
  res
}

fn comma_kv<'a, E: Er<&'a [u8]>>(i: &'a[u8]) -> IResult<&'a [u8], (&'a str, JsonValue), E> {
  let (i, _) = sp(i)?;
  let (i, _) = char(',')(i)?;
  key_value(i)
}

fn hash_internal<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], HashMap<&'a str, JsonValue>, E> {
  //println!("hash_internal");
  let res = match key_value(input) {
    Err(Err::Error(_)) => Ok((input, HashMap::default())),
    Err(e) => Err(e),
    Ok((i, (key, value))) => {
      let mut map = HashMap::default();
      map.insert(key, value);

      let mut input = i;
      loop {
        //match do_parse!(input, sp >> char!(',') >> kv: key_value >> (kv)) {
        //match do_parse!(input, char!(',') >> kv: key_value >> (kv)) {
        //match preceded(input, char(','), key_value) {
        match comma_kv(input) {
          Err(Err::Error(_)) => break Ok((input, map)),
          Err(e) => break Err(e),
          Ok((i, (key, value))) => {
            map.insert(key, value);
            input = i;
          }
        }
      }
    }
  };
  //println!("hash_internal(\"{}\") returned {:?}", str::from_utf8(input).unwrap(), res);
  res

}

/*named!(
  hash<HashMap<&str, JsonValue>>,
*/
fn hash<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], HashMap<&'a str, JsonValue>, E> {
    let res = delimited(input,
      char('{'),
      hash_internal,
      //preceded!(sp, char!('}'))
      char('}')
    );
    //println!("hash(\"{}\") returned {:?}", str::from_utf8(input).unwrap(), res);
    res
}

fn json_value<'a, E: Er<&'a [u8]>>(input: &'a[u8]) -> IResult<&'a[u8], JsonValue, E> {
  //println!("json_value");
  let res = or(input, &[
   &|i| { map(i, string, JsonValue::Str) },
   &|i| { map(i, float, JsonValue::Num) },
   &|i| { map(i, array, JsonValue::Array) },
   &|i| { map(i, hash, JsonValue::Object) },
   &|i| { map(i, boolean, JsonValue::Boolean) },
  ]);
  //println!("json_value({}) -> {:?}", str::from_utf8(input).unwrap(), res);
  res
}

fn root<'a, E: Er<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], JsonValue, E> {
  //println!("root");
  let res = or(input, &[
   &|i| { map(i, array, JsonValue::Array) },
   &|i| { map(i, hash, JsonValue::Object) },
  ]);
  //println!("root({}) -> {:?}", str::from_utf8(input).unwrap(), res);
  res
}

fn basic(b: &mut Bencher) {
  let data = b"{\"a\":42,\"b\":[\"x\",\"y\",12],\"c\":{\"hello\":\"world\"}};";
  //let data = b"{}";

  b.bytes = data.len() as u64;
  parse::<(&[u8], u32)>(b, &data[..])
}

fn verbose(b: &mut Bencher) {
  let data = b"{\"a\":42,\"b\":[\"x\",\"y\",12],\"c\":{\"hello\":\"world\"}};";
  //let data = b"{}";

  b.bytes = data.len() as u64;
  parse::<Verbose<&[u8]>>(b, &data[..])
}

fn parse<'a, E: Er<&'a[u8]>+Debug>(b: &mut Bencher, buffer: &'a[u8]) {
  let res: IResult<_, _, E> = root(buffer);
  //println!("res: {:?}", res);
  assert!(res.is_ok());

  b.iter(|| {
    let mut buf = black_box(buffer);
    let res: IResult<_, _, E> = root(buf);
    match res {
      Ok((i, o)) => {
        return o;
      }
      Err(err) => {
        panic!("got parsing error: {:?}", err);
      },
    }
  });
}


benchmark_group!(json, basic, verbose);
benchmark_main!(json);
