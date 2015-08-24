//! Deserialization code.

use std::io::{self, Read};
use std::mem::size_of;
use byteorder::ReadBytesExt;
use byteorder::NativeEndian as E;

use super::{
	SIGNATURE, FORMAT, VERSION, DATA, TEST_INT, TEST_NUMBER,
	Int, Size, Instruction, Integer, Number,
	Constant, Upvalue, LocalVar, Debug, Function,
};

/// Deserialize bytecode into a `Function`.
pub fn read_file<R: Read>(read: R) -> io::Result<Function> {
	let mut reader = Reader { out: read };
	try!(reader.read_header());
	try!(reader.out.read_u8()); // discard upvals header
	reader.read_function()
}

struct Reader<R: Read> {
	out: R,
}

fn invalid<T, S: Into<Box<::std::error::Error + Send + Sync>>>(s: S) -> io::Result<T> {
	Err(io::Error::new(io::ErrorKind::InvalidInput, s))
}

macro_rules! check {
	($get:expr, $want:expr, $note:expr) => {{
		let get = $get;
		let want = $want;
		if get != want {
			return Err(io::Error::new(io::ErrorKind::InvalidInput, format!(
				"invalid {}, expected {:?} but got {:?}",
				$note, want, get,
			)));
		}
	}}
}

impl<R: Read> Reader<R> {
	fn read_all(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
		let mut start = 0;
		let len = buf.len();
		while start < len {
			let n = try!(self.out.read(&mut buf[start..]));
			if n == 0 {
				return invalid("unexpected EOF");
			}
			start += n;
		}
		Ok(())
	}

	fn read_header(&mut self) -> io::Result<()> {
		let mut buffer = [0u8; 6];
		try!(self.read_all(&mut buffer[..4]));
		check!(&buffer[..4], SIGNATURE, "signature");
		check!(try!(self.out.read_u8()), VERSION, "version");
		check!(try!(self.out.read_u8()), FORMAT, "format");
		try!(self.read_all(&mut buffer));
		check!(&buffer, DATA, "test data");
		check!(try!(self.out.read_u8()), size_of::<Int>() as u8, "sizeof(int)");
		check!(try!(self.out.read_u8()), size_of::<Size>() as u8, "sizeof(size_t)");
		check!(try!(self.out.read_u8()), size_of::<Instruction>() as u8, "sizeof(Instruction)");
		check!(try!(self.out.read_u8()), size_of::<Integer>() as u8, "sizeof(Integer)");
		check!(try!(self.out.read_u8()), size_of::<Number>() as u8, "sizeof(Number)");
		check!(try!(self.out.read_i64::<E>()), TEST_INT, "test integer");
		check!(try!(self.out.read_f64::<E>()), TEST_NUMBER, "test number");
		Ok(())
	}

	fn read_function(&mut self) -> io::Result<Function> {
		Ok(Function {
			source: try!(self.read_string()),
			line_start: try!(self.out.read_i32::<E>()),
			line_end: try!(self.out.read_i32::<E>()),
			num_params: try!(self.out.read_u8()),
			is_vararg: try!(self.out.read_u8()) != 0,
			max_stack_size: try!(self.out.read_u8()),
			code: try!(self.read_vec(|this| Ok(try!(this.out.read_u32::<E>())))),
			constants: try!(self.read_vec(|this| Ok(match try!(this.out.read_u8()) {
				0x00 => Constant::Nil,
				0x01 => Constant::Boolean(try!(this.out.read_u8()) != 0),
				0x03 => Constant::Float(try!(this.out.read_f64::<E>())),
				0x13 => Constant::Int(try!(this.out.read_i64::<E>())),
				0x04 => Constant::ShortString(try!(this.read_string())),
				0x14 => Constant::LongString(try!(this.read_string())),
				o => return invalid(format!("unknown constant type {}", o)),
			}))),
			upvalues: try!(self.read_vec(|this| {
				let stack = try!(this.out.read_u8());
				let idx = try!(this.out.read_u8());
				Ok(match stack {
					0 => Upvalue::Outer(idx),
					_ => Upvalue::Stack(idx),
				})
			})),
			protos: try!(self.read_vec(|this| this.read_function())),
			debug: Debug {
				lineinfo: try!(self.read_vec(|this| Ok(try!(this.out.read_i32::<E>())))),
				localvars: try!(self.read_vec(|this| Ok(LocalVar {
					name: try!(this.read_string()),
					start_pc: try!(this.out.read_i32::<E>()),
					end_pc: try!(this.out.read_i32::<E>()),
				}))),
				upvalues: try!(self.read_vec(|this| this.read_string())),
			},
		})
	}

	#[inline]
	fn read_vec<F, T>(&mut self, f: F) -> io::Result<Vec<T>>
		where F: Fn(&mut Self) -> io::Result<T>
	{
		let len = try!(self.out.read_u32::<E>());
		(0..len).map(|_| f(self)).collect()
	}

	fn read_string(&mut self) -> io::Result<String> {
		let first = try!(self.out.read_u8());
		if first == 0 {
			Ok(String::new())
		} else {
			let len = if first < 0xff {
				first as usize
			} else {
				try!(self.out.read_u32::<E>()) as usize
			} - 1;
			let mut buffer = vec![0u8; len];
			try!(self.read_all(&mut buffer));
			// TODO: May need to return a Vec<u8> rather than String
			match String::from_utf8(buffer) {
				Ok(s) => Ok(s),
				Err(_) => invalid("not utf8"),
			}
		}
	}
}
