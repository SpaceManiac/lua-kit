//! Toolkit for working with serialized Lua functions and bytecode.
//!
//! Synced to Lua 5.3.

extern crate byteorder;

pub mod bytecode;
mod write;
mod read;

pub use write::write_file;
pub use read::read_file;

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
