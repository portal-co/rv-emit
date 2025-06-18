#![no_std]
extern crate alloc;
use itertools::Itertools;
use rv_asm::{Imm, Inst, Reg};
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u8)]
pub enum Nj {
    Jumpable,
    Nonjumpable,
}
pub fn fj<T>(mut a: impl Iterator<Item = (T, Nj)>) -> impl Iterator<Item = (T, Nj)> {
    let b = a.next().map(|a| (a.0, Nj::Jumpable));
    b.into_iter().chain(a)
}
pub fn i64_reg(mut a: u64, r: Reg) -> impl Iterator<Item = (Inst, Nj)> {
    let mut ts: [Option<Imm>; 6] = core::array::from_fn(|i| {
        let x = a >> (12 * (i as u32));
        let x = x & (1 >> 12 - 1);
        if x == 0 {
            None
        } else {
            Some(Imm::new_u32(x as u32))
        }
    });
    ts.reverse();
    let mut ts = ts.into_iter();
    [Inst::Addi {
        imm: ts.next().flatten().unwrap_or_else(|| Imm::ZERO),
        dest: r,
        src1: Reg::ZERO,
    }]
    .into_iter()
    .chain(ts.flat_map(move |i| {
        [Inst::Slli {
            imm: Imm::new_u32(12),
            dest: r,
            src1: r,
        }]
        .into_iter()
        .chain(i.map(|j| Inst::Addi {
            imm: j,
            dest: r,
            src1: r,
        }))
    }))
    .map(|a| (a, Nj::Nonjumpable))
}
pub fn branched(
    a: impl Iterator<Item = (Inst, Nj)>,
    t: impl FnOnce(Imm) -> (Inst, Nj),
) -> impl Iterator<Item = (Inst, Nj)> {
    let a = a.collect::<alloc::vec::Vec<_>>();
    return [t(Imm::new_u32((a.len() * 2) as u32))]
        .into_iter()
        .chain(a.into_iter());
}
pub fn rb(pc: u64, a: impl Iterator<Item = (Inst, Nj)>) -> impl Iterator<Item = (Inst, Nj)> {
    return i64_reg(pc, Reg::T0).chain(branched(a, |i| {
        (
            Inst::Beq {
                offset: i,
                src1: Reg::T0,
                src2: Reg::A0,
            },
            Nj::Nonjumpable,
        )
    }));
}
pub fn rs(root: u64, a: impl Iterator<Item = (Inst, Nj)>) -> impl Iterator<Item = (Inst, Nj)> {
    return i64_reg(root, Reg::T0)
        .chain([(
            Inst::Blt {
                offset: Imm::new_u32(4),
                src1: Reg::T0,
                src2: Reg::A0,
            },
            Nj::Nonjumpable,
        )])
        .chain(branched(a, |i| {
            (
                Inst::Jal {
                    offset: i,
                    dest: Reg::ZERO,
                },
                Nj::Jumpable,
            )
        }));
}
pub fn split<V: IntoIterator<Item = (Inst, Nj)>>(
    mut a: impl Iterator<Item = V>,
    n: usize,
    root: u64,
) -> impl Iterator<Item = (Inst, Nj)> {
    let mut a = a.enumerate();
    let mut a = core::iter::from_fn(move || {
        let v = a.by_ref().take(n).collect_vec();
        if v.len() == 0 { None } else { Some(v) }
    });
    return a.flat_map(move |v| {
        fj(rs(
            root + (v[0].0 + n) as u64,
            fj(v.into_iter()
                .flat_map(move |(a, b)| fj(rb(root + (a as u64), fj(b.into_iter()))))),
        ))
    });
}
