# arcref

A smart pointer type that can wrap either a `&'static T` or an `Arc<T>`.

## Equality

`ArcRef<T>` implements `PartialEq` by checking pointer identity first. If two
`ArcRef`s point at the same backing allocation or the same `&'static` reference,
they compare equal without calling `T`'s `PartialEq` implementation. Otherwise,
equality falls back to comparing the dereferenced values.

This means cloned `ArcRef`s are always equal to each other, and equality remains
reflexive even for wrapped types whose `PartialEq` is not, such as `f64::NAN`.

See the `examples/` folder for example usage.
