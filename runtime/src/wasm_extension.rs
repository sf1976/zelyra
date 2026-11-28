//! Capability-free WebAssembly v1 profile for pure, bounded i32 functions.
//! The only input channel is the argument vector. There are no host imports.
pub const MAX_WASM_EXTENSION_BYTES: usize = 16 * 1024;
const MAX_ARGS: usize = 16;
const MAX_LOCALS: usize = 32;
const MAX_STACK: usize = 64;
const MAX_INSTRUCTIONS: usize = 1024;
const I32: u8 = 0x7f;

type ExtensionResult<T> = Result<T, &'static str>;

pub fn run_wasm_extension(module: &[u8], export: &str, args: &[i32]) -> ExtensionResult<i32> {
    if module.len() > MAX_WASM_EXTENSION_BYTES {
        return Err("extension module exceeds its size limit");
    }
    let mut module = Reader(module);
    if module.take(8)? != b"\0asm\x01\0\0\0" {
        return Err("unsupported WebAssembly header");
    }
    let mut signature = module.section(1)?;
    signature.expect_u32(1)?;
    signature.expect_byte(0x60)?;
    let parameters = signature.u32()?;
    if parameters > MAX_ARGS || parameters != args.len() {
        return Err("extension argument count is invalid");
    }
    for _ in 0..parameters {
        signature.expect_byte(I32)?;
    }
    signature.expect_u32(1)?;
    signature.expect_byte(I32)?;
    signature.end()?;

    let mut functions = module.section(3)?;
    functions.expect_u32(1)?;
    functions.expect_u32(0)?;
    functions.end()?;

    let mut exports = module.section(7)?;
    exports.expect_u32(1)?;
    let length = exports.u32()?;
    if !(1..=64).contains(&length) {
        return Err("invalid extension export name length");
    }
    let name =
        std::str::from_utf8(exports.take(length)?).map_err(|_| "invalid UTF-8 export name")?;
    if name != export {
        return Err("extension export does not match the requested entry");
    }
    exports.expect_byte(0)?;
    exports.expect_u32(0)?;
    exports.end()?;

    let mut code = module.section(10)?;
    code.expect_u32(1)?;
    let length = code.u32()?;
    let body = Reader(code.take(length)?);
    code.end()?;
    module.end()?;
    evaluate(body, args)
}

fn evaluate(mut code: Reader<'_>, args: &[i32]) -> ExtensionResult<i32> {
    let mut locals = args.to_vec();
    let groups = code.u32()?;
    if groups > MAX_LOCALS {
        return Err("extension local group limit exceeded");
    }
    for _ in 0..groups {
        let count = code.u32()?;
        code.expect_byte(I32)?;
        if count > MAX_LOCALS - locals.len() {
            return Err("extension local limit exceeded");
        }
        locals.resize(locals.len() + count, 0);
    }

    let mut stack: Vec<i32> = Vec::with_capacity(MAX_STACK + 1);
    for _ in 0..MAX_INSTRUCTIONS {
        match code.byte()? {
            0x0b => {
                code.end()?;
                return match stack.as_slice() {
                    [result] => Ok(*result),
                    _ => Err("extension function must return one i32"),
                };
            }
            0x1a => {
                pop(&mut stack)?;
            }
            0x1b => {
                let condition = pop(&mut stack)?;
                let (left, right) = operands(&mut stack)?;
                stack.push(if condition == 0 { right } else { left });
            }
            0x20 => {
                let value = locals
                    .get(code.u32()?)
                    .ok_or("invalid extension local index")?;
                stack.push(*value);
            }
            opcode @ (0x21 | 0x22) => {
                let local = locals
                    .get_mut(code.u32()?)
                    .ok_or("invalid extension local index")?;
                *local = pop(&mut stack)?;
                if opcode == 0x22 {
                    stack.push(*local);
                }
            }
            0x41 => stack.push(code.i32()?),
            0x45 => {
                let value = pop(&mut stack)?;
                stack.push(i32::from(value == 0));
            }
            opcode @ (0x46..=0x4f | 0x6a..=0x6c | 0x71..=0x73) => {
                let (left, right) = operands(&mut stack)?;
                stack.push(binary(opcode, left, right));
            }
            _ => return Err("instruction is outside the capability-free extension profile"),
        }
        if stack.len() > MAX_STACK {
            return Err("extension operand stack limit exceeded");
        }
    }
    Err("extension instruction limit exceeded")
}

fn pop(stack: &mut Vec<i32>) -> ExtensionResult<i32> {
    stack.pop().ok_or("extension operand stack underflow")
}

fn operands(stack: &mut Vec<i32>) -> ExtensionResult<(i32, i32)> {
    let right = pop(stack)?;
    let left = pop(stack)?;
    Ok((left, right))
}

fn binary(opcode: u8, left: i32, right: i32) -> i32 {
    match opcode {
        0x46 => i32::from(left == right),
        0x47 => i32::from(left != right),
        0x48 => i32::from(left < right),
        0x49 => i32::from((left as u32) < (right as u32)),
        0x4a => i32::from(left > right),
        0x4b => i32::from((left as u32) > (right as u32)),
        0x4c => i32::from(left <= right),
        0x4d => i32::from((left as u32) <= (right as u32)),
        0x4e => i32::from(left >= right),
        0x4f => i32::from((left as u32) >= (right as u32)),
        0x6a => left.wrapping_add(right),
        0x6b => left.wrapping_sub(right),
        0x6c => left.wrapping_mul(right),
        0x71 => left & right,
        0x72 => left | right,
        0x73 => left ^ right,
        _ => unreachable!("validated binary extension opcode"),
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> ExtensionResult<&'a [u8]> {
        let (value, rest) = self
            .0
            .split_at_checked(size)
            .ok_or("truncated extension module")?;
        self.0 = rest;
        Ok(value)
    }

    fn byte(&mut self) -> ExtensionResult<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> ExtensionResult<usize> {
        let value =
            u32::try_from(self.leb128(false)?).map_err(|_| "invalid unsigned extension integer")?;
        Ok(value as usize)
    }

    fn i32(&mut self) -> ExtensionResult<i32> {
        i32::try_from(self.leb128(true)?).map_err(|_| "invalid signed extension integer")
    }

    fn leb128(&mut self, signed: bool) -> ExtensionResult<i64> {
        let mut value = 0i64;
        for shift in (0..35).step_by(7) {
            let byte = self.byte()?;
            value |= i64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                if signed && byte & 0x40 != 0 {
                    value |= (-1i64) << (shift + 7);
                }
                return Ok(value);
            }
        }
        Err("extension integer exceeds five bytes")
    }

    fn section(&mut self, id: u8) -> ExtensionResult<Self> {
        self.expect_byte(id)?;
        let length = self.u32()?;
        Ok(Self(self.take(length)?))
    }

    fn expect_u32(&mut self, expected: usize) -> ExtensionResult<()> {
        if self.u32()? == expected {
            Ok(())
        } else {
            Err("unsupported extension module structure")
        }
    }

    fn expect_byte(&mut self, expected: u8) -> ExtensionResult<()> {
        if self.byte()? == expected {
            Ok(())
        } else {
            Err("unsupported extension section or type; imports, memory and host effects are denied")
        }
    }

    fn end(&self) -> ExtensionResult<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err("unexpected trailing bytes in extension module")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (module (func (export "add") (param i32 i32) (result i32)
    //   local.get 0 local.get 1 i32.add))
    const ADD: &[u8] = b"\0asm\x01\0\0\0\x01\x07\x01\x60\x02\x7f\x7f\x01\x7f\x03\x02\x01\0\x07\x07\x01\x03add\0\0\x0a\x09\x01\x07\0\x20\0\x20\x01\x6a\x0b";

    #[test]
    fn computes_a_pure_i32_export() {
        assert_eq!(run_wasm_extension(ADD, "add", &[5, -2]), Ok(3));
        assert_eq!(run_wasm_extension(ADD, "add", &[i32::MAX, 1]), Ok(i32::MIN));
        assert!(run_wasm_extension(ADD, "missing", &[1, 2]).is_err());
        assert!(run_wasm_extension(ADD, "add", &[1]).is_err());
    }

    #[test]
    fn rejects_imports_memory_calls_and_malformed_modules() {
        for section in [&[2, 1, 0][..], &[5, 3, 1, 0, 1], &[8, 1, 0]] {
            let mut module = ADD.to_vec();
            module.splice(17..17, section.iter().copied());
            assert!(run_wasm_extension(&module, "add", &[1, 2]).is_err());
        }
        for prefix in 0..ADD.len() {
            assert!(run_wasm_extension(&ADD[..prefix], "add", &[1, 2]).is_err());
        }
        let mut module = ADD.to_vec();
        let opcode = module.len() - 2;
        module[opcode] = 0x10; // call
        assert!(run_wasm_extension(&module, "add", &[1, 2]).is_err());
        assert!(Reader(&[0xff, 0xff, 0xff, 0xff, 0x0f]).i32().is_err());
        assert!(Reader(&[0xff, 0xff, 0xff, 0xff, 0x10]).u32().is_err());
    }
}
