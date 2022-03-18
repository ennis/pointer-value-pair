# Pointer-and-value wrapper type for Rust

This crate provides the `PointerAndValue` type, a wrapper over a raw `*const T` pointer that also packs a small integer value
in the alignment bits, with the guarantee that `mem::size_of::<PointerAndValue<T>>() == mem::size_of::<*const T>()`.

It is inspired by [llvm::PointerIntPair](https://llvm.org/doxygen/classllvm_1_1PointerIntPair.html) from LLVM, and [TfPointerAndBits](`https://graphics.pixar.com/usd/release/api/class_tf_pointer_and_bits.html`) from USD.

It also provides `Cow`, which is similar to [std::borrow::Cow]() but stores either `&'a T` or `Box<T>`, and is guaranteed to be the same size as `*const T`.

## TODOs and limitations
- This currently does not work with pointers to zero-sized types because `mem::align_of` returns a minimum alignment of 1.
- Support dynamically-sized types