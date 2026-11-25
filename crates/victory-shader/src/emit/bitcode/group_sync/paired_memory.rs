use super::super::{Bin, Cast, Op, Rmw, Term, TypeDef, V, pred};
use super::Proof;
use std::collections::HashSet;

impl Proof<'_> {
    /// Explore the post-optimization control flow for each of the 32 host
    /// lanes. Unknown branch conditions can take either edge. Each possible
    /// LDS access must stay in the allocation, and every plain store must
    /// have its own dword across both logical halves.
    pub(super) fn paired_memory(&self) -> bool {
        let mut writes = HashSet::new();
        for lane in 0..32 {
            let mut values = Lane {
                proof: self,
                lane,
                known: vec![None; self.m.insts.len()],
            };
            let mut seen = vec![false; self.m.blocks.len()];
            let mut pending = vec![0];
            while let Some(b) = pending.pop() {
                if std::mem::replace(&mut seen[b], true) {
                    continue;
                }
                let block = &self.m.blocks[b];
                for &inst in &block.body {
                    let op = &self.m.insts[inst as usize].op;
                    let ptr = match op {
                        Op::Load(ptr)
                        | Op::Store(ptr, _)
                        | Op::AtomicRmw(_, ptr, _)
                        | Op::CmpXchg(ptr, ..) => *ptr,
                        _ => continue,
                    };
                    if !matches!(
                        self.m.types[self.m.type_of(ptr).0 as usize],
                        TypeDef::Ptr(_, 3)
                    ) {
                        continue;
                    }
                    let Some((global, len, index)) = self.paired_address(ptr) else {
                        return false;
                    };
                    let slot = values.eval(index).filter(|&word| word < len);
                    match op {
                        Op::Store(..) => {
                            if !slot.is_some_and(|word| writes.insert((global, word))) {
                                return false;
                            }
                        }
                        Op::Load(..)
                        | Op::AtomicRmw(Rmw::Add | Rmw::Or | Rmw::UMin | Rmw::UMax, ..) => {
                            if slot.is_none() && !self.guarded_lds_access(b, index, len) {
                                return false;
                            }
                        }
                        Op::AtomicRmw(..) | Op::CmpXchg(..) => return false,
                        _ => {}
                    }
                }
                match block.term {
                    Some(Term::Br(to)) => pending.push(to),
                    Some(Term::CondBr(cond, yes, no)) => match values.eval(cond) {
                        Some(0) => pending.push(no),
                        Some(_) => pending.push(yes),
                        None => pending.extend([yes, no]),
                    },
                    Some(Term::Ret) => {}
                    None => return false,
                }
            }
        }
        true
    }

    fn paired_address(&self, ptr: V) -> Option<(u32, u32, V)> {
        let (_, Op::Gep(array, V::Global(global), indices)) = self.def(ptr)? else {
            return None;
        };
        let TypeDef::Array(len, elem) = self.m.types[array.0 as usize] else {
            return None;
        };
        let [zero, index] = indices.as_slice() else {
            return None;
        };
        (self.m.globals[*global as usize].space == 3
            && self.m.types[elem.0 as usize] == TypeDef::Int(32)
            && self.m.int_value(*zero) == Some(0))
        .then_some((*global, len, *index))
    }

    fn guarded_lds_access(&self, access: usize, index: V, len: u32) -> bool {
        self.m.blocks.iter().enumerate().any(|(b, block)| {
            let Some(Term::CondBr(cond, yes, no)) = block.term else {
                return false;
            };
            yes != no
                && self.cfg.pred[yes].as_slice() == [b]
                && self.cfg.dom.has(access, yes)
                && self.contains_lds_bound(cond, index, len)
        })
    }

    fn contains_lds_bound(&self, cond: V, index: V, len: u32) -> bool {
        match self.def(cond) {
            Some((_, Op::Cmp(pred::IULT, actual, bound))) => {
                *actual == index && self.m.int_value(*bound) == Some(i64::from(len))
            }
            Some((_, Op::Bin(Bin::And, a, b))) => {
                self.contains_lds_bound(*a, index, len) || self.contains_lds_bound(*b, index, len)
            }
            _ => false,
        }
    }
}

struct Lane<'a, 'm> {
    proof: &'a Proof<'m>,
    lane: u32,
    known: Vec<Option<Option<u32>>>,
}

impl Lane<'_, '_> {
    fn eval(&mut self, v: V) -> Option<u32> {
        let m = self.proof.m;
        let TypeDef::Int(bits @ 1..=32) = m.types[m.type_of(v).0 as usize] else {
            return None;
        };
        let mask = u32::MAX >> (32 - bits);
        let V::Inst(i) = v else {
            return m.int_value(v).map(|value| value as u32 & mask);
        };
        let i = i as usize;
        if let Some(value) = self.known[i] {
            return value;
        }
        // A recursive phi may depend on the iteration. Treat it as unknown.
        self.known[i] = Some(None);
        let result = self.eval_op(&m.insts[i].op, bits, mask).map(|v| v & mask);
        self.known[i] = Some(result);
        result
    }

    fn eval_op(&mut self, op: &Op, bits: u32, mask: u32) -> Option<u32> {
        let m = self.proof.m;
        match op {
            Op::Call(f, args)
                if m.funcs[*f as usize].name == "dx.op.waveGetLaneIndex"
                    && matches!(args.as_slice(), [code] if m.int_value(*code) == Some(111)) =>
            {
                Some(self.lane)
            }
            Op::Bin(op, a, b) => {
                let (a, b) = (self.eval(*a), self.eval(*b));
                if *op == Bin::And && (a == Some(0) || b == Some(0)) {
                    return Some(0);
                }
                if *op == Bin::Or && (a == Some(mask) || b == Some(mask)) {
                    return Some(mask);
                }
                let (a, b) = (a?, b?);
                match op {
                    Bin::Add => Some(a.wrapping_add(b)),
                    Bin::Sub => Some(a.wrapping_sub(b)),
                    Bin::Mul => Some(a.wrapping_mul(b)),
                    Bin::UDiv => a.checked_div(b),
                    Bin::URem => a.checked_rem(b),
                    Bin::And => Some(a & b),
                    Bin::Or => Some(a | b),
                    Bin::Xor => Some(a ^ b),
                    Bin::Shl if b < bits => Some(a << b),
                    Bin::LShr if b < bits => Some(a >> b),
                    _ => None,
                }
            }
            Op::Cmp(op, a, b) if (pred::EQ..=pred::IULE).contains(op) => {
                let (a, b) = (self.eval(*a)?, self.eval(*b)?);
                Some(u32::from(match *op {
                    pred::EQ => a == b,
                    pred::NE => a != b,
                    pred::IUGT => a > b,
                    pred::IUGE => a >= b,
                    pred::IULT => a < b,
                    pred::IULE => a <= b,
                    _ => return None,
                }))
            }
            Op::Cast(Cast::Trunc | Cast::ZExt, src) => self.eval(*src),
            Op::Select(cond, yes, no) => match self.eval(*cond) {
                Some(0) => self.eval(*no),
                Some(_) => self.eval(*yes),
                None => {
                    let yes = self.eval(*yes)?;
                    (self.eval(*no) == Some(yes)).then_some(yes)
                }
            },
            Op::Phi(incoming) => {
                let first = self.eval(incoming.first()?.0)?;
                incoming
                    .iter()
                    .skip(1)
                    .all(|(value, _)| self.eval(*value) == Some(first))
                    .then_some(first)
            }
            _ => None,
        }
    }
}
