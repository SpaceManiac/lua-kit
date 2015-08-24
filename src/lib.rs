//! Toolkit for working with serialized Lua functions and bytecode.
//!
//! Synced to Lua 5.3.

extern crate byteorder;

use std::io::{self, Write};
use std::mem::size_of;
use byteorder::WriteBytesExt;
use byteorder::NativeEndian as E;

pub mod bytecode;

/// Signature to mark Lua bytecode files.
pub const SIGNATURE: &'static [u8] = b"\x1bLua";
/// The Lua version, in the form `(MAJOR << 4) | MINOR`.
pub const VERSION: u8 = 0x53;
/// The Lua bytecode format.
pub const FORMAT: u8 = 0;
/// Test text to catch translation errors.
pub const DATA: &'static [u8] = b"\x19\x93\r\n\x1a\n";
/// A test integer to know endianness.
pub const TEST_INT: Integer = 0x5678;
/// A test floating-point number to know endianness.
pub const TEST_NUMBER: Number = 370.5;

/// The bytecode's C `int` type.
pub type Int = i32;
/// The bytecodes' C `size_t` type.
pub type Size = u32;
/// The bytecode's `Instruction` type.
pub type Instruction = u32;
/// The bytecode's `Integer` type.
pub type Integer = i64;
/// The bytecode's `Number` (floating-point) type.
pub type Number = f64;

/// An entry in the constant pool.
#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
	/// The value `nil`.
	Nil,
	/// A boolean.
	Boolean(bool),
	/// A floating-point number.
	Float(Number),
	/// An integer.
	Int(Integer),
	/// A short string.
	ShortString(String),
	/// A long string. Behaves the same as `ShortString`.
	LongString(String),
}

/// An entry in the upvalue table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Upvalue {
	/// An upvalue in the outer function's list.
	Outer(u8),
	/// An upvalue on the stack (register).
	Stack(u8),
}

/// An entry in the local variable debug table.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalVar {
	/// The local variable's name.
	pub name: String,
	/// The instruction at which the local variable is introduced.
	pub start_pc: Int,
	/// The instruction at which the local variable goes out of scope.
	pub end_pc: Int,
}

/// Optional debugging information for a function.
#[derive(Clone, Debug, PartialEq)]
pub struct Debug {
	/// The line number of each bytecode instruction.
	pub lineinfo: Vec<Int>,
	/// The names and scopes of local variables.
	pub localvars: Vec<LocalVar>,
	/// The names of upvalues.
	pub upvalues: Vec<String>,
}

impl Debug {
	/// A new, empty debug info.
	pub fn none() -> Debug {
		Debug {
			lineinfo: vec![],
			localvars: vec![],
			upvalues: vec![],
		}
	}
}

/// A Lua function prototype.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
	/// The source filename of the function. May be empty.
	pub source: String,
	/// The start line number of the function.
	pub line_start: Int,
	/// The end line number of the function.
	pub line_end: Int,
	/// The number of fixed parameters the function takes.
	pub num_params: u8,
	/// Whether the function accepts a variable number of arguments.
	pub is_vararg: bool,
	/// The number of registers needed by the function.
	pub max_stack_size: u8,
	/// The function's code.
	pub code: Vec<Instruction>,
	/// The function's constant table.
	pub constants: Vec<Constant>,
	/// The upvalue information of the function.
	pub upvalues: Vec<Upvalue>,
	/// The function's contained function prototypes.
	pub protos: Vec<Function>,
	/// Debugging information for the function.
	pub debug: Debug,
}

/// Serialize a `Function` to bytecode.
pub fn write_file<W: Write>(write: W, function: &Function) -> io::Result<()> {
	let mut writer = Writer { out: write };
	try!(writer.write_header());
	try!(writer.out.write_u8(function.upvalues.len() as u8));
	writer.write_function(function)
}

struct Writer<W: Write> {
	out: W,
}

impl<W: Write> Writer<W> {
	fn write_header(&mut self) -> io::Result<()> {
		try!(self.out.write_all(SIGNATURE));
		try!(self.out.write_u8(VERSION));
		try!(self.out.write_u8(FORMAT));
		try!(self.out.write_all(DATA));
		try!(self.out.write_u8(size_of::<Int>() as u8));
		try!(self.out.write_u8(size_of::<Size>() as u8));
		try!(self.out.write_u8(size_of::<Instruction>() as u8));
		try!(self.out.write_u8(size_of::<Integer>() as u8));
		try!(self.out.write_u8(size_of::<Number>() as u8));
		try!(self.out.write_i64::<E>(TEST_INT));
		try!(self.out.write_f64::<E>(TEST_NUMBER));
		Ok(())
	}

	fn write_function(&mut self, function: &Function) -> io::Result<()> {
		try!(self.write_string(&function.source));
		try!(self.out.write_i32::<E>(function.line_start));
		try!(self.out.write_i32::<E>(function.line_end));
		try!(self.out.write_u8(function.num_params));
		try!(self.out.write_u8(if function.is_vararg { 1 } else { 0 }));
		try!(self.out.write_u8(function.max_stack_size));
		
		try!(self.out.write_u32::<E>(function.code.len() as u32));
		for &ins in &function.code {
			try!(self.out.write_u32::<E>(ins));
		}
		try!(self.out.write_u32::<E>(function.constants.len() as u32));
		for cons in &function.constants {
			match cons {
				&Constant::Nil => try!(self.out.write_u8(0x00)),
				&Constant::Boolean(b) => try!(self.out.write_all(&[0x01, if b { 1 } else { 0 }])),
				&Constant::Float(n) => {
					try!(self.out.write_u8(0x03));
					try!(self.out.write_f64::<E>(n));
				}
				&Constant::Int(n) => {
					try!(self.out.write_u8(0x13));
					try!(self.out.write_i64::<E>(n));
				}
				&Constant::ShortString(ref s) => {
					try!(self.out.write_u8(0x04));
					try!(self.write_string(s));
				}
				&Constant::LongString(ref s) => {
					try!(self.out.write_u8(0x14));
					try!(self.write_string(s));
				}
			}
		}
		try!(self.out.write_u32::<E>(function.upvalues.len() as u32));
		for upval in &function.upvalues {
			try!(match upval {
				&Upvalue::Outer(idx) => self.out.write_all(&[0, idx]),
				&Upvalue::Stack(idx) => self.out.write_all(&[1, idx]),
			});
		}
		try!(self.out.write_u32::<E>(function.protos.len() as u32));
		for proto in &function.protos {
			try!(self.write_function(proto));
		}
		// debug
		try!(self.out.write_u32::<E>(function.debug.lineinfo.len() as u32));
		for &line in &function.debug.lineinfo {
			try!(self.out.write_i32::<E>(line));
		}
		try!(self.out.write_u32::<E>(function.debug.localvars.len() as u32));
		for var in &function.debug.localvars {
			try!(self.write_string(&var.name));
			try!(self.out.write_i32::<E>(var.start_pc));
			try!(self.out.write_i32::<E>(var.end_pc));
		}
		try!(self.out.write_u32::<E>(function.debug.upvalues.len() as u32));
		for upval in &function.debug.upvalues {
			try!(self.write_string(upval));
		}
		Ok(())
	}

	fn write_string(&mut self, string: &str) -> io::Result<()> {
		if string.len() == 0 {
			try!(self.out.write_u8(0))
		} else {
			if string.len() >= 0xff {
				try!(self.out.write_u8(0xff));
				try!(self.out.write_u32::<E>(string.len() as u32));
			} else {
				try!(self.out.write_u8(string.len() as u8 + 1));
			}
			try!(self.out.write_all(string.as_bytes()))
		}
		Ok(())
	}
}
