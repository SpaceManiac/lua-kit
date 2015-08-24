//! Tools for bytecode generation.

const BITRK: u32 = 1 << 8;

/// A slot which is either a register (`R`) or constant (`K`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RK {
	/// A register index.
	R(u8),
	/// A constant table index.
	K(u8),
}

impl RK {
	/// Convert a number to an `RK`.
	pub fn decode(value: u32) -> RK {
		if value & BITRK != 0 {
			RK::K((value & !BITRK) as u8)
		} else {
			RK::R(value as u8)
		}
	}
	/// Convert this `RK` to a number.
	pub fn encode(&self) -> u32 {
		match self {
			&RK::R(r) => r as u32,
			&RK::K(k) => (k as u32) | BITRK,
		}
	}
}

/// Encode an instruction with `A`, `B`, and `C` parameters.
pub fn encode(op: Opcode, a: u8, b: u32, c: u32) -> u32 {
	(op as u32) | ((a as u32) << 6) | ((c & 0x1ff) << 14) | ((b & 0x1ff) << 23)
}

/// Encode an instruction with `A` and `Bx` parameters.
pub fn encode_bx(op: Opcode, a: u8, bx: u32) -> u32 {
	(op as u32) | ((a as u32) << 6) | ((bx & 0x3ffff) << 14)
}

/// Encode an instruction with `A` and `sBx` parameters.
pub fn encode_sbx(op: Opcode, a: u8, sbx: i32) -> u32 {
	(op as u32) | ((a as u32) << 6) | ((((sbx + 0x20000) as u32) & 0x3ffff) << 14)
}

/// Encode an instruction with an `Ax` parameter.
pub fn encode_ax(op: Opcode, ax: u32) -> u32 {
	(op as u32) | ((ax & 0x3ffffff) << 6)
}

// LSB 6      8         9         9  MSB
// |------|--------|---------|---------|
// |opcode|   A    |    C    |    B    |
// |opcode|   A    |     Bx or sBx     |
// |opcode|             Ax             |
// All 'skips' (pc++) assume that next instruction is a jump.

/// A Lua opcode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Opcode { // Args   Action
	Move,     // A B    R(A) := R(B)
	LoadK,    // A Bx   R(A) := Kst(Bx)
	LoadKX,   // A      R(A) := Kst(extra arg)
	// ^- the next 'instruction' is always EXTRAARG.
	LoadBool, // A B C  R(A) := (Bool)B; if (C) pc++
	LoadNil,  // A B    R(A), R(A+1), ..., R(A+B) := nil

	GetUpval, // A B C  R(A) := UpValue[B]
	GetTabUp, // A B C  R(A) := UpValue[B][RK(C)]
	GetTable, // A B C  R(A) := R(B)[RK(C)]

	SetTabUp, // A B C  UpValue[A][RK(B)] := RK(C)
	SetUpval, // A B    UpValue[B] := R(A)
	SetTable, // A B C  R(A)[RK(B)] := RK(C)

	NewTable, // A B C  R(A) := {} (size = B,C)

	Self_,     // A B C  R(A+1) := R(B); R(A) := R(B)[RK(C)]

	Add,      // A B C  R(A) := RK(B) + RK(C)
	Sub,      // A B C  R(A) := RK(B) - RK(C)
	Mul,      // A B C  R(A) := RK(B) * RK(C)
	Mod,      // A B C  R(A) := RK(B) % RK(C)
	Pow,      // A B C  R(A) := RK(B) ^ RK(C)
	Div,      // A B C  R(A) := RK(B) / RK(C)
	IntDiv,   // A B C  R(A) := RK(B) // RK(C)
	BinAnd,   // A B C  R(A) := RK(B) & RK(C)
	BinOr,    // A B C  R(A) := RK(B) | RK(C)
	BinXor,   // A B C  R(A) := RK(B) ~ RK(C)
	ShLeft,   // A B C  R(A) := RK(B) << RK(C)
	ShRight,  // A B C  R(A) := RK(B) >> RK(C)
	UnMinus,  // A B    R(A) := -R(B)
	BinNot,   // A B    R(A) := ~R(B)
	Not,      // A B    R(A) := not R(B)
	Len,      // A B    R(A) := length of R(B)

	Concat,   // A B C  R(A) := R(B).. ... ..R(C)

	Jump,     // A sBx  pc += sBx; if(A) close all upvalues >= R(A - 1)
	Eq,       // A B C  if ((RK(B) == RK(C)) ~= A) then pc++
	Less,     // A B C  if ((RK(B) <  RK(C)) ~= A) then pc++
	LessEq,   // A B C  if ((RK(B) <= RK(C)) ~= A) then pc++
	// ^- A specifies what condition the test should accept (true or false).
	Test,     // A   C 	if not (R(A) <=> C) then pc++
	TestSet,  // A B C  if (R(B) <=> C) then R(A) := R(B) else pc++

	Call,     // A B C  R(A), ... ,R(A+C-2) := R(A)(R(A+1), ... ,R(A+B-1))
	// ^- if (B == 0) then B = top. If (C == 0), then 'top' is
    //    set to last_result+1, so next open instruction (OP_CALL, OP_RETURN,
    //    OP_SETLIST) may use 'top'.
	TailCall, // A B C  return R(A)(R(A+1), ... ,R(A+B-1))
	Return,   // A B    return R(A), ... ,R(A+B-2)
	// ^- if (B == 0) then return up to 'top'

	ForLoop,  // A sBx  R(A)+=R(A+2); if R(A) <?= R(A+1) then { pc+=sBx; R(A+3)=R(A) }
	ForPrep,  // A sBx  R(A)-=R(A+2); pc+=sBx
	TForCall, // A   C  R(A+3), ... ,R(A+2+C) := R(A)(R(A+1), R(A+2));
	TForLoop, // A sBx  if R(A+1) ~= nil then { R(A)=R(A+1); pc += sBx }
	SetList,  // A B C  R(A)[(C-1)*FPF+i] := R(A+i), 1 <= i <= B
	// ^- if (B == 0) then B = 'top'; if (C == 0) then next
	//    'instruction' is EXTRAARG(real C).
	Closure,  // A Bx   R(A) := closure(KPROTO[Bx])
	VarArg,   // A B    R(A), R(A+1), ..., R(A+B-2) = vararg
	// ^- if (B == 0) then use actual number of varargs and
    //    set top (like in OP_CALL with C == 0).
	ExtraArg, // Ax     extra (larger) argument for previous opcode
}
