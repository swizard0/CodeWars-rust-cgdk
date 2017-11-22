use std::marker;
use std::mem;
use std::num::Wrapping as w;

pub mod isaac {
    use std::slice;
    use std::iter::repeat;
    use std::num::Wrapping as w;
    use std::fmt;

    use super::{Rng, SeedableRng, Rand, w32, w64};

    const RAND_SIZE_LEN: usize = 8;
    const RAND_SIZE: u32 = 1 << RAND_SIZE_LEN;
    const RAND_SIZE_USIZE: usize = 1 << RAND_SIZE_LEN;

    #[derive(Copy)]
    pub struct IsaacRng {
        cnt: u32,
        rsl: [w32; RAND_SIZE_USIZE],
        mem: [w32; RAND_SIZE_USIZE],
        a: w32,
        b: w32,
        c: w32,
    }

    static EMPTY: IsaacRng = IsaacRng {
        cnt: 0,
        rsl: [w(0); RAND_SIZE_USIZE],
        mem: [w(0); RAND_SIZE_USIZE],
        a: w(0), b: w(0), c: w(0),
    };

    impl IsaacRng {
        #[allow(dead_code)]
        pub fn new_unseeded() -> IsaacRng {
            let mut rng = EMPTY;
            rng.init(false);
            rng
        }

        fn init(&mut self, use_rsl: bool) {
            let mut a = w(0x9e3779b9);
            let mut b = a;
            let mut c = a;
            let mut d = a;
            let mut e = a;
            let mut f = a;
            let mut g = a;
            let mut h = a;

            macro_rules! mix {
                () => {{
                    a=a^(b<<11); d=d+a; b=b+c;
                    b=b^(c>>2);  e=e+b; c=c+d;
                    c=c^(d<<8);  f=f+c; d=d+e;
                    d=d^(e>>16); g=g+d; e=e+f;
                    e=e^(f<<10); h=h+e; f=f+g;
                    f=f^(g>>4);  a=a+f; g=g+h;
                    g=g^(h<<8);  b=b+g; h=h+a;
                    h=h^(a>>9);  c=c+h; a=a+b;
                }}
            }

            for _ in 0..4 {
                mix!();
            }

            if use_rsl {
                macro_rules! memloop {
                    ($arr:expr) => {{
                        for i in (0..RAND_SIZE_USIZE/8).map(|i| i * 8) {
                            a=a+$arr[i  ]; b=b+$arr[i+1];
                            c=c+$arr[i+2]; d=d+$arr[i+3];
                            e=e+$arr[i+4]; f=f+$arr[i+5];
                            g=g+$arr[i+6]; h=h+$arr[i+7];
                            mix!();
                            self.mem[i  ]=a; self.mem[i+1]=b;
                            self.mem[i+2]=c; self.mem[i+3]=d;
                            self.mem[i+4]=e; self.mem[i+5]=f;
                            self.mem[i+6]=g; self.mem[i+7]=h;
                        }
                    }}
                }

                memloop!(self.rsl);
                memloop!(self.mem);
            } else {
                for i in (0..RAND_SIZE_USIZE/8).map(|i| i * 8) {
                    mix!();
                    self.mem[i  ]=a; self.mem[i+1]=b;
                    self.mem[i+2]=c; self.mem[i+3]=d;
                    self.mem[i+4]=e; self.mem[i+5]=f;
                    self.mem[i+6]=g; self.mem[i+7]=h;
                }
            }

            self.isaac();
        }

        /// Refills the output buffer (`self.rsl`)
        #[inline]
        fn isaac(&mut self) {
            self.c = self.c + w(1);
            // abbreviations
            let mut a = self.a;
            let mut b = self.b + self.c;

            const MIDPOINT: usize = RAND_SIZE_USIZE / 2;

            macro_rules! ind {
                ($x:expr) => ( self.mem[($x >> 2usize).0 as usize & (RAND_SIZE_USIZE - 1)] )
            }

            let r = [(0, MIDPOINT), (MIDPOINT, 0)];
            for &(mr_offset, m2_offset) in r.iter() {

                macro_rules! rngstepp {
                    ($j:expr, $shift:expr) => {{
                        let base = $j;
                        let mix = a << $shift;

                        let x = self.mem[base  + mr_offset];
                        a = (a ^ mix) + self.mem[base + m2_offset];
                        let y = ind!(x) + a + b;
                        self.mem[base + mr_offset] = y;

                        b = ind!(y >> RAND_SIZE_LEN) + x;
                        self.rsl[base + mr_offset] = b;
                    }}
                }

                macro_rules! rngstepn {
                    ($j:expr, $shift:expr) => {{
                        let base = $j;
                        let mix = a >> $shift;

                        let x = self.mem[base  + mr_offset];
                        a = (a ^ mix) + self.mem[base + m2_offset];
                        let y = ind!(x) + a + b;
                        self.mem[base + mr_offset] = y;

                        b = ind!(y >> RAND_SIZE_LEN) + x;
                        self.rsl[base + mr_offset] = b;
                    }}
                }

                for i in (0..MIDPOINT/4).map(|i| i * 4) {
                    rngstepp!(i + 0, 13);
                    rngstepn!(i + 1, 6);
                    rngstepp!(i + 2, 2);
                    rngstepn!(i + 3, 16);
                }
            }

            self.a = a;
            self.b = b;
            self.cnt = RAND_SIZE;
        }
    }

    // Cannot be derived because [u32; 256] does not implement Clone
    impl Clone for IsaacRng {
        fn clone(&self) -> IsaacRng {
            *self
        }
    }

    impl Rng for IsaacRng {
        #[inline]
        fn next_u32(&mut self) -> u32 {
            if self.cnt == 0 {
                // make some more numbers
                self.isaac();
            }
            self.cnt -= 1;

            debug_assert!(self.cnt < RAND_SIZE);

            self.rsl[(self.cnt % RAND_SIZE) as usize].0
        }
    }

    impl<'a> SeedableRng<&'a [u32]> for IsaacRng {
        fn reseed(&mut self, seed: &'a [u32]) {
            // make the seed into [seed[0], seed[1], ..., seed[seed.len()
            // - 1], 0, 0, ...], to fill rng.rsl.
            let seed_iter = seed.iter().map(|&x| x).chain(repeat(0u32));

            for (rsl_elem, seed_elem) in self.rsl.iter_mut().zip(seed_iter) {
                *rsl_elem = w(seed_elem);
            }
            self.cnt = 0;
            self.a = w(0);
            self.b = w(0);
            self.c = w(0);

            self.init(true);
        }

        fn from_seed(seed: &'a [u32]) -> IsaacRng {
            let mut rng = EMPTY;
            rng.reseed(seed);
            rng
        }
    }

    impl Rand for IsaacRng {
        fn rand<R: Rng>(other: &mut R) -> IsaacRng {
            let mut ret = EMPTY;
            unsafe {
                let ptr = ret.rsl.as_mut_ptr() as *mut u8;

                let slice = slice::from_raw_parts_mut(ptr, RAND_SIZE_USIZE * 4);
                other.fill_bytes(slice);
            }
            ret.cnt = 0;
            ret.a = w(0);
            ret.b = w(0);
            ret.c = w(0);

            ret.init(true);
            return ret;
        }
    }

    impl fmt::Debug for IsaacRng {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "IsaacRng {{}}")
        }
    }

    const RAND_SIZE_64_LEN: usize = 8;
    const RAND_SIZE_64: usize = 1 << RAND_SIZE_64_LEN;

    #[derive(Copy)]
    pub struct Isaac64Rng {
        cnt: usize,
        rsl: [w64; RAND_SIZE_64],
        mem: [w64; RAND_SIZE_64],
        a: w64,
        b: w64,
        c: w64,
    }

    static EMPTY_64: Isaac64Rng = Isaac64Rng {
        cnt: 0,
        rsl: [w(0); RAND_SIZE_64],
        mem: [w(0); RAND_SIZE_64],
        a: w(0), b: w(0), c: w(0),
    };

    impl Isaac64Rng {
        #[allow(dead_code)]
        pub fn new_unseeded() -> Isaac64Rng {
            let mut rng = EMPTY_64;
            rng.init(false);
            rng
        }

        fn init(&mut self, use_rsl: bool) {
            macro_rules! init {
                ($var:ident) => (
                    let mut $var = w(0x9e3779b97f4a7c13);
                )
            }
            init!(a); init!(b); init!(c); init!(d);
            init!(e); init!(f); init!(g); init!(h);

            macro_rules! mix {
                () => {{
                    a=a-e; f=f^(h>>9);  h=h+a;
                    b=b-f; g=g^(a<<9);  a=a+b;
                    c=c-g; h=h^(b>>23); b=b+c;
                    d=d-h; a=a^(c<<15); c=c+d;
                    e=e-a; b=b^(d>>14); d=d+e;
                    f=f-b; c=c^(e<<20); e=e+f;
                    g=g-c; d=d^(f>>17); f=f+g;
                    h=h-d; e=e^(g<<14); g=g+h;
                }}
            }

            for _ in 0..4 {
                mix!();
            }

            if use_rsl {
                macro_rules! memloop {
                    ($arr:expr) => {{
                        for i in (0..RAND_SIZE_64 / 8).map(|i| i * 8) {
                            a=a+$arr[i  ]; b=b+$arr[i+1];
                            c=c+$arr[i+2]; d=d+$arr[i+3];
                            e=e+$arr[i+4]; f=f+$arr[i+5];
                            g=g+$arr[i+6]; h=h+$arr[i+7];
                            mix!();
                            self.mem[i  ]=a; self.mem[i+1]=b;
                            self.mem[i+2]=c; self.mem[i+3]=d;
                            self.mem[i+4]=e; self.mem[i+5]=f;
                            self.mem[i+6]=g; self.mem[i+7]=h;
                        }
                    }}
                }

                memloop!(self.rsl);
                memloop!(self.mem);
            } else {
                for i in (0..RAND_SIZE_64 / 8).map(|i| i * 8) {
                    mix!();
                    self.mem[i  ]=a; self.mem[i+1]=b;
                    self.mem[i+2]=c; self.mem[i+3]=d;
                    self.mem[i+4]=e; self.mem[i+5]=f;
                    self.mem[i+6]=g; self.mem[i+7]=h;
                }
            }

            self.isaac64();
        }

        /// Refills the output buffer (`self.rsl`)
        fn isaac64(&mut self) {
            self.c = self.c + w(1);
            // abbreviations
            let mut a = self.a;
            let mut b = self.b + self.c;
            const MIDPOINT: usize =  RAND_SIZE_64 / 2;
            const MP_VEC: [(usize, usize); 2] = [(0,MIDPOINT), (MIDPOINT, 0)];
            macro_rules! ind {
                ($x:expr) => {
                    *self.mem.get_unchecked((($x >> 3usize).0 as usize) & (RAND_SIZE_64 - 1))
                }
            }

            for &(mr_offset, m2_offset) in MP_VEC.iter() {
                for base in (0..MIDPOINT / 4).map(|i| i * 4) {

                    macro_rules! rngstepp {
                        ($j:expr, $shift:expr) => {{
                            let base = base + $j;
                            let mix = a ^ (a << $shift);
                            let mix = if $j == 0 {!mix} else {mix};

                            unsafe {
                                let x = *self.mem.get_unchecked(base + mr_offset);
                                a = mix + *self.mem.get_unchecked(base + m2_offset);
                                let y = ind!(x) + a + b;
                                *self.mem.get_unchecked_mut(base + mr_offset) = y;

                                b = ind!(y >> RAND_SIZE_64_LEN) + x;
                                *self.rsl.get_unchecked_mut(base + mr_offset) = b;
                            }
                        }}
                    }

                    macro_rules! rngstepn {
                        ($j:expr, $shift:expr) => {{
                            let base = base + $j;
                            let mix = a ^ (a >> $shift);
                            let mix = if $j == 0 {!mix} else {mix};

                            unsafe {
                                let x = *self.mem.get_unchecked(base + mr_offset);
                                a = mix + *self.mem.get_unchecked(base + m2_offset);
                                let y = ind!(x) + a + b;
                                *self.mem.get_unchecked_mut(base + mr_offset) = y;

                                b = ind!(y >> RAND_SIZE_64_LEN) + x;
                                *self.rsl.get_unchecked_mut(base + mr_offset) = b;
                            }
                        }}
                    }

                    rngstepp!(0, 21);
                    rngstepn!(1, 5);
                    rngstepp!(2, 12);
                    rngstepn!(3, 33);
                }
            }

            self.a = a;
            self.b = b;
            self.cnt = RAND_SIZE_64;
        }
    }

    // Cannot be derived because [u32; 256] does not implement Clone
    impl Clone for Isaac64Rng {
        fn clone(&self) -> Isaac64Rng {
            *self
        }
    }

    impl Rng for Isaac64Rng {
        // FIXME #7771: having next_u32 like this should be unnecessary
        #[inline]
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        #[inline]
        fn next_u64(&mut self) -> u64 {
            if self.cnt == 0 {
                // make some more numbers
                self.isaac64();
            }
            self.cnt -= 1;

            // See corresponding location in IsaacRng.next_u32 for
            // explanation.
            debug_assert!(self.cnt < RAND_SIZE_64);
            self.rsl[(self.cnt % RAND_SIZE_64) as usize].0
        }
    }

    impl<'a> SeedableRng<&'a [u64]> for Isaac64Rng {
        fn reseed(&mut self, seed: &'a [u64]) {
            // make the seed into [seed[0], seed[1], ..., seed[seed.len()
            // - 1], 0, 0, ...], to fill rng.rsl.
            let seed_iter = seed.iter().map(|&x| x).chain(repeat(0u64));

            for (rsl_elem, seed_elem) in self.rsl.iter_mut().zip(seed_iter) {
                *rsl_elem = w(seed_elem);
            }
            self.cnt = 0;
            self.a = w(0);
            self.b = w(0);
            self.c = w(0);

            self.init(true);
        }

        /// Create an ISAAC random number generator with a seed. This can
        /// be any length, although the maximum number of elements used is
        /// 256 and any more will be silently ignored. A generator
        /// constructed with a given seed will generate the same sequence
        /// of values as all other generators constructed with that seed.
        fn from_seed(seed: &'a [u64]) -> Isaac64Rng {
            let mut rng = EMPTY_64;
            rng.reseed(seed);
            rng
        }
    }

    impl Rand for Isaac64Rng {
        fn rand<R: Rng>(other: &mut R) -> Isaac64Rng {
            let mut ret = EMPTY_64;
            unsafe {
                let ptr = ret.rsl.as_mut_ptr() as *mut u8;

                let slice = slice::from_raw_parts_mut(ptr, RAND_SIZE_64 * 8);
                other.fill_bytes(slice);
            }
            ret.cnt = 0;
            ret.a = w(0);
            ret.b = w(0);
            ret.c = w(0);

            ret.init(true);
            return ret;
        }
    }

    impl fmt::Debug for Isaac64Rng {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Isaac64Rng {{}}")
        }
    }
}


pub use self::isaac::{IsaacRng, Isaac64Rng};

// #[cfg(target_pointer_width = "32")]
// use self::isaac::IsaacRng as IsaacWordRng;
// #[cfg(target_pointer_width = "64")]
// use self::isaac::Isaac64Rng as IsaacWordRng;

pub mod distributions {
    use std::marker;

    use super::{Rng, Rand};

    pub mod range {
        use std::num::Wrapping as w;

        use super::super::Rng;
        use super::{Sample, IndependentSample};

        #[derive(Clone, Copy, Debug)]
        pub struct Range<X> {
            low: X,
            range: X,
            accept_zone: X
        }

        impl<X: SampleRange + PartialOrd> Range<X> {
            pub fn new(low: X, high: X) -> Range<X> {
                assert!(low < high, "Range::new called with `low >= high`");
                SampleRange::construct_range(low, high)
            }
        }

        impl<Sup: SampleRange> Sample<Sup> for Range<Sup> {
            #[inline]
            fn sample<R: Rng>(&mut self, rng: &mut R) -> Sup { self.ind_sample(rng) }
        }
        impl<Sup: SampleRange> IndependentSample<Sup> for Range<Sup> {
            fn ind_sample<R: Rng>(&self, rng: &mut R) -> Sup {
                SampleRange::sample_range(self, rng)
            }
        }

        pub trait SampleRange : Sized {
            fn construct_range(low: Self, high: Self) -> Range<Self>;
            fn sample_range<R: Rng>(r: &Range<Self>, rng: &mut R) -> Self;
        }

        macro_rules! integer_impl {
            ($ty:ty, $unsigned:ident) => {
                impl SampleRange for $ty {
                    #[inline]
                    fn construct_range(low: $ty, high: $ty) -> Range<$ty> {
                        let range = (w(high as $unsigned) - w(low as $unsigned)).0;
                        let unsigned_max: $unsigned = ::std::$unsigned::MAX;
                        let zone = unsigned_max - unsigned_max % range;

                        Range {
                            low: low,
                            range: range as $ty,
                            accept_zone: zone as $ty
                        }
                    }

                    #[inline]
                    fn sample_range<R: Rng>(r: &Range<$ty>, rng: &mut R) -> $ty {
                        loop {
                            let v = rng.gen::<$unsigned>();
                            if v < r.accept_zone as $unsigned {
                                return (w(r.low) + w((v % r.range as $unsigned) as $ty)).0;
                            }
                        }
                    }
                }
            }
        }

        integer_impl! { i8, u8 }
        integer_impl! { i16, u16 }
        integer_impl! { i32, u32 }
        integer_impl! { i64, u64 }
        integer_impl! { isize, usize }
        integer_impl! { u8, u8 }
        integer_impl! { u16, u16 }
        integer_impl! { u32, u32 }
        integer_impl! { u64, u64 }
        integer_impl! { usize, usize }

        macro_rules! float_impl {
            ($ty:ty) => {
                impl SampleRange for $ty {
                    fn construct_range(low: $ty, high: $ty) -> Range<$ty> {
                        Range {
                            low: low,
                            range: high - low,
                            accept_zone: 0.0 // unused
                        }
                    }
                    fn sample_range<R: Rng>(r: &Range<$ty>, rng: &mut R) -> $ty {
                        r.low + r.range * rng.gen::<$ty>()
                    }
                }
            }
        }

        float_impl! { f32 }
        float_impl! { f64 }
    }

    pub use self::range::Range;
    // pub use self::gamma::{Gamma, ChiSquared, FisherF, StudentT};
    // pub use self::normal::{Normal, LogNormal};
    // pub use self::exponential::Exp;

    // pub mod range;
    // pub mod gamma;
    // pub mod normal;
    // pub mod exponential;

    pub trait Sample<Support> {
        fn sample<R: Rng>(&mut self, rng: &mut R) -> Support;
    }

    // FIXME maybe having this separate is overkill (the only reason is to
    // take &self rather than &mut self)? or maybe this should be the
    // trait called `Sample` and the other should be `DependentSample`.
    pub trait IndependentSample<Support>: Sample<Support> {
        fn ind_sample<R: Rng>(&self, &mut R) -> Support;
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct RandSample<Sup> {
        _marker: marker::PhantomData<fn() -> Sup>,
    }

    impl<Sup> Copy for RandSample<Sup> {}
    impl<Sup> Clone for RandSample<Sup> {
        fn clone(&self) -> Self { *self }
    }

    impl<Sup: Rand> Sample<Sup> for RandSample<Sup> {
        fn sample<R: Rng>(&mut self, rng: &mut R) -> Sup { self.ind_sample(rng) }
    }

    impl<Sup: Rand> IndependentSample<Sup> for RandSample<Sup> {
        fn ind_sample<R: Rng>(&self, rng: &mut R) -> Sup {
            rng.gen()
        }
    }

    impl<Sup> RandSample<Sup> {
        #[allow(dead_code)]
        pub fn new() -> RandSample<Sup> {
            RandSample { _marker: marker::PhantomData }
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct Weighted<T> {
        pub weight: u32,
        pub item: T,
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct WeightedChoice<'a, T:'a> {
        items: &'a mut [Weighted<T>],
        weight_range: Range<u32>
    }

    impl<'a, T: Clone> WeightedChoice<'a, T> {
        #[allow(dead_code)]
        pub fn new(items: &'a mut [Weighted<T>]) -> WeightedChoice<'a, T> {
            // strictly speaking, this is subsumed by the total weight == 0 case
            assert!(!items.is_empty(), "WeightedChoice::new called with no items");

            let mut running_total: u32 = 0;

            // we convert the list from individual weights to cumulative
            // weights so we can binary search. This *could* drop elements
            // with weight == 0 as an optimisation.
            for item in items.iter_mut() {
                running_total = match running_total.checked_add(item.weight) {
                    Some(n) => n,
                    None => panic!("WeightedChoice::new called with a total weight \
                                    larger than a u32 can contain")
                };

                item.weight = running_total;
            }
            assert!(running_total != 0, "WeightedChoice::new called with a total weight of 0");

            WeightedChoice {
                items: items,
                // we're likely to be generating numbers in this range
                // relatively often, so might as well cache it
                weight_range: Range::new(0, running_total)
            }
        }
    }

    impl<'a, T: Clone> Sample<T> for WeightedChoice<'a, T> {
        fn sample<R: Rng>(&mut self, rng: &mut R) -> T { self.ind_sample(rng) }
    }

    impl<'a, T: Clone> IndependentSample<T> for WeightedChoice<'a, T> {
        fn ind_sample<R: Rng>(&self, rng: &mut R) -> T {
            // we want to find the first element that has cumulative
            // weight > sample_weight, which we do by binary since the
            // cumulative weights of self.items are sorted.

            // choose a weight in [0, total_weight)
            let sample_weight = self.weight_range.ind_sample(rng);

            // short circuit when it's the first item
            if sample_weight < self.items[0].weight {
                return self.items[0].item.clone();
            }

            let mut idx = 0;
            let mut modifier = self.items.len();

            // now we know that every possibility has an element to the
            // left, so we can just search for the last element that has
            // cumulative weight <= sample_weight, then the next one will
            // be "it". (Note that this greatest element will never be the
            // last element of the vector, since sample_weight is chosen
            // in [0, total_weight) and the cumulative weight of the last
            // one is exactly the total weight.)
            while modifier > 1 {
                let i = idx + modifier / 2;
                if self.items[i].weight <= sample_weight {
                    // we're small, so look to the right, but allow this
                    // exact element still.
                    idx = i;
                    // we need the `/ 2` to round up otherwise we'll drop
                    // the trailing elements when `modifier` is odd.
                    modifier += 1;
                } else {
                    // otherwise we're too big, so go left. (i.e. do
                    // nothing)
                }
                modifier /= 2;
            }
            return self.items[idx + 1].item.clone();
        }
    }
}

use self::distributions::{Range, IndependentSample};
use self::distributions::range::SampleRange;

mod rand_impls {
    use std::char;
    use std::mem;

    use super::{Rand,Rng};

    impl Rand for isize {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> isize {
            if mem::size_of::<isize>() == 4 {
                rng.gen::<i32>() as isize
            } else {
                rng.gen::<i64>() as isize
            }
        }
    }

    impl Rand for i8 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> i8 {
            rng.next_u32() as i8
        }
    }

    impl Rand for i16 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> i16 {
            rng.next_u32() as i16
        }
    }

    impl Rand for i32 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> i32 {
            rng.next_u32() as i32
        }
    }

    impl Rand for i64 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> i64 {
            rng.next_u64() as i64
        }
    }

    #[cfg(feature = "i128_support")]
    impl Rand for i128 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> i128 {
            rng.gen::<u128>() as i128
        }
    }

    impl Rand for usize {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> usize {
            if mem::size_of::<usize>() == 4 {
                rng.gen::<u32>() as usize
            } else {
                rng.gen::<u64>() as usize
            }
        }
    }

    impl Rand for u8 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> u8 {
            rng.next_u32() as u8
        }
    }

    impl Rand for u16 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> u16 {
            rng.next_u32() as u16
        }
    }

    impl Rand for u32 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> u32 {
            rng.next_u32()
        }
    }

    impl Rand for u64 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> u64 {
            rng.next_u64()
        }
    }

    #[cfg(feature = "i128_support")]
    impl Rand for u128 {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> u128 {
            ((rng.next_u64() as u128) << 64) | (rng.next_u64() as u128)
        }
    }


    macro_rules! float_impls {
        ($mod_name:ident, $ty:ty, $mantissa_bits:expr, $method_name:ident) => {
            mod $mod_name {
                use super::super::{Rand, Rng, Open01, Closed01};

                const SCALE: $ty = (1u64 << $mantissa_bits) as $ty;

                impl Rand for $ty {
                    /// Generate a floating point number in the half-open
                    /// interval `[0,1)`.
                    ///
                    /// See `Closed01` for the closed interval `[0,1]`,
                    /// and `Open01` for the open interval `(0,1)`.
                    #[inline]
                    fn rand<R: Rng>(rng: &mut R) -> $ty {
                        rng.$method_name()
                    }
                }
                impl Rand for Open01<$ty> {
                    #[inline]
                    fn rand<R: Rng>(rng: &mut R) -> Open01<$ty> {
                        // add a small amount (specifically 2 bits below
                        // the precision of f64/f32 at 1.0), so that small
                        // numbers are larger than 0, but large numbers
                        // aren't pushed to/above 1.
                        Open01(rng.$method_name() + 0.25 / SCALE)
                    }
                }
                impl Rand for Closed01<$ty> {
                    #[inline]
                    fn rand<R: Rng>(rng: &mut R) -> Closed01<$ty> {
                        // rescale so that 1.0 - epsilon becomes 1.0
                        // precisely.
                        Closed01(rng.$method_name() * SCALE / (SCALE - 1.0))
                    }
                }
            }
        }
    }
    float_impls! { f64_rand_impls, f64, 53, next_f64 }
    float_impls! { f32_rand_impls, f32, 24, next_f32 }

    impl Rand for char {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> char {
            // a char is 21 bits
            const CHAR_MASK: u32 = 0x001f_ffff;
            loop {
                // Rejection sampling. About 0.2% of numbers with at most
                // 21-bits are invalid codepoints (surrogates), so this
                // will succeed first go almost every time.
                match char::from_u32(rng.next_u32() & CHAR_MASK) {
                    Some(c) => return c,
                    None => {}
                }
            }
        }
    }

    impl Rand for bool {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> bool {
            rng.gen::<u8>() & 1 == 1
        }
    }

    macro_rules! tuple_impl {
        // use variables to indicate the arity of the tuple
        ($($tyvar:ident),* ) => {
            // the trailing commas are for the 1 tuple
            impl<
                $( $tyvar : Rand ),*
                > Rand for ( $( $tyvar ),* , ) {

                    #[inline]
                    fn rand<R: Rng>(_rng: &mut R) -> ( $( $tyvar ),* , ) {
                        (
                            // use the $tyvar's to get the appropriate number of
                            // repeats (they're not actually needed)
                            $(
                                _rng.gen::<$tyvar>()
                            ),*
                                ,
                        )
                    }
                }
        }
    }

    impl Rand for () {
        #[inline]
        fn rand<R: Rng>(_: &mut R) -> () { () }
    }
    tuple_impl!{A}
    tuple_impl!{A, B}
    tuple_impl!{A, B, C}
    tuple_impl!{A, B, C, D}
    tuple_impl!{A, B, C, D, E}
    tuple_impl!{A, B, C, D, E, F}
    tuple_impl!{A, B, C, D, E, F, G}
    tuple_impl!{A, B, C, D, E, F, G, H}
    tuple_impl!{A, B, C, D, E, F, G, H, I}
    tuple_impl!{A, B, C, D, E, F, G, H, I, J}
    tuple_impl!{A, B, C, D, E, F, G, H, I, J, K}
    tuple_impl!{A, B, C, D, E, F, G, H, I, J, K, L}

    macro_rules! array_impl {
        {$n:expr, $t:ident, $($ts:ident,)*} => {
            array_impl!{($n - 1), $($ts,)*}

            impl<T> Rand for [T; $n] where T: Rand {
                #[inline]
                fn rand<R: Rng>(_rng: &mut R) -> [T; $n] {
                    [_rng.gen::<$t>(), $(_rng.gen::<$ts>()),*]
                }
            }
        };
        {$n:expr,} => {
            impl<T> Rand for [T; $n] {
                fn rand<R: Rng>(_rng: &mut R) -> [T; $n] { [] }
            }
        };
    }

    array_impl!{32, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T,}

    impl<T:Rand> Rand for Option<T> {
        #[inline]
        fn rand<R: Rng>(rng: &mut R) -> Option<T> {
            if rng.gen() {
                Some(rng.gen())
            } else {
                None
            }
        }
    }
}

// pub mod reseeding;
// pub mod os;
// pub mod read;

#[allow(bad_style)]
type w64 = w<u64>;
#[allow(bad_style)]
type w32 = w<u32>;

pub trait Rand : Sized {
    fn rand<R: Rng>(rng: &mut R) -> Self;
}

pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | (self.next_u32() as u64)
    }
    fn next_f32(&mut self) -> f32 {
        const UPPER_MASK: u32 = 0x3F800000;
        const LOWER_MASK: u32 = 0x7FFFFF;
        let tmp = UPPER_MASK | (self.next_u32() & LOWER_MASK);
        let result: f32 = unsafe { mem::transmute(tmp) };
        result - 1.0
    }
    fn next_f64(&mut self) -> f64 {
        const UPPER_MASK: u64 = 0x3FF0000000000000;
        const LOWER_MASK: u64 = 0xFFFFFFFFFFFFF;
        let tmp = UPPER_MASK | (self.next_u64() & LOWER_MASK);
        let result: f64 = unsafe { mem::transmute(tmp) };
        result - 1.0
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut count = 0;
        let mut num = 0;
        for byte in dest.iter_mut() {
            if count == 0 {
                num = self.next_u64();
                count = 8;
            }

            *byte = (num & 0xff) as u8;
            num >>= 8;
            count -= 1;
        }
    }
    #[inline(always)]
    fn gen<T: Rand>(&mut self) -> T where Self: Sized {
        Rand::rand(self)
    }
    fn gen_iter<'a, T: Rand>(&'a mut self) -> Generator<'a, T, Self> where Self: Sized {
        Generator { rng: self, _marker: marker::PhantomData }
    }
    fn gen_range<T: PartialOrd + SampleRange>(&mut self, low: T, high: T) -> T where Self: Sized {
        assert!(low < high, "Rng.gen_range called with low >= high");
        Range::new(low, high).ind_sample(self)
    }
    fn gen_weighted_bool(&mut self, n: u32) -> bool where Self: Sized {
        n <= 1 || self.gen_range(0, n) == 0
    }
    fn gen_ascii_chars<'a>(&'a mut self) -> AsciiGenerator<'a, Self> where Self: Sized {
        AsciiGenerator { rng: self }
    }
    fn choose<'a, T>(&mut self, values: &'a [T]) -> Option<&'a T> where Self: Sized {
        if values.is_empty() {
            None
        } else {
            Some(&values[self.gen_range(0, values.len())])
        }
    }
    fn choose_mut<'a, T>(&mut self, values: &'a mut [T]) -> Option<&'a mut T> where Self: Sized {
        if values.is_empty() {
            None
        } else {
            let len = values.len();
            Some(&mut values[self.gen_range(0, len)])
        }
    }
    fn shuffle<T>(&mut self, values: &mut [T]) where Self: Sized {
        let mut i = values.len();
        while i >= 2 {
            // invariant: elements with index >= i have been locked in place.
            i -= 1;
            // lock element i in place.
            values.swap(i, self.gen_range(0, i + 1));
        }
    }
}

impl<'a, R: ?Sized> Rng for &'a mut R where R: Rng {
    fn next_u32(&mut self) -> u32 {
        (**self).next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        (**self).next_u64()
    }

    fn next_f32(&mut self) -> f32 {
        (**self).next_f32()
    }

    fn next_f64(&mut self) -> f64 {
        (**self).next_f64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        (**self).fill_bytes(dest)
    }
}

impl<R: ?Sized> Rng for Box<R> where R: Rng {
    fn next_u32(&mut self) -> u32 {
        (**self).next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        (**self).next_u64()
    }

    fn next_f32(&mut self) -> f32 {
        (**self).next_f32()
    }

    fn next_f64(&mut self) -> f64 {
        (**self).next_f64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        (**self).fill_bytes(dest)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Generator<'a, T, R:'a> {
    rng: &'a mut R,
    _marker: marker::PhantomData<fn() -> T>,
}

impl<'a, T: Rand, R: Rng> Iterator for Generator<'a, T, R> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        Some(self.rng.gen())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AsciiGenerator<'a, R:'a> {
    rng: &'a mut R,
}

impl<'a, R: Rng> Iterator for AsciiGenerator<'a, R> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        const GEN_ASCII_STR_CHARSET: &'static [u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
              abcdefghijklmnopqrstuvwxyz\
              0123456789";
        Some(*self.rng.choose(GEN_ASCII_STR_CHARSET).unwrap() as char)
    }
}

pub trait SeedableRng<Seed>: Rng {
    fn reseed(&mut self, Seed);

    fn from_seed(seed: Seed) -> Self;
}

#[allow(missing_copy_implementations)]
#[derive(Clone, Debug)]
pub struct XorShiftRng {
    x: w32,
    y: w32,
    z: w32,
    w: w32,
}

impl XorShiftRng {
    #[allow(dead_code)]
    pub fn new_unseeded() -> XorShiftRng {
        XorShiftRng {
            x: w(0x193a6754),
            y: w(0xa8a7d469),
            z: w(0x97830e05),
            w: w(0x113ba7bb),
        }
    }
}

impl Rng for XorShiftRng {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        let x = self.x;
        let t = x ^ (x << 11);
        self.x = self.y;
        self.y = self.z;
        self.z = self.w;
        let w_ = self.w;
        self.w = w_ ^ (w_ >> 19) ^ (t ^ (t >> 8));
        self.w.0
    }
}

impl SeedableRng<[u32; 4]> for XorShiftRng {
    fn reseed(&mut self, seed: [u32; 4]) {
        assert!(!seed.iter().all(|&x| x == 0),
                "XorShiftRng.reseed called with an all zero seed.");

        self.x = w(seed[0]);
        self.y = w(seed[1]);
        self.z = w(seed[2]);
        self.w = w(seed[3]);
    }

    fn from_seed(seed: [u32; 4]) -> XorShiftRng {
        assert!(!seed.iter().all(|&x| x == 0),
                "XorShiftRng::from_seed called with an all zero seed.");

        XorShiftRng {
            x: w(seed[0]),
            y: w(seed[1]),
            z: w(seed[2]),
            w: w(seed[3]),
        }
    }
}

impl Rand for XorShiftRng {
    fn rand<R: Rng>(rng: &mut R) -> XorShiftRng {
        let mut tuple: (u32, u32, u32, u32) = rng.gen();
        while tuple == (0, 0, 0, 0) {
            tuple = rng.gen();
        }
        let (x, y, z, w_) = tuple;
        XorShiftRng { x: w(x), y: w(y), z: w(z), w: w(w_) }
    }
}

#[derive(Debug)]
pub struct Open01<F>(pub F);

#[derive(Debug)]
pub struct Closed01<F>(pub F);

#[allow(dead_code)]
pub fn sample<T, I, R>(rng: &mut R, iterable: I, amount: usize) -> Vec<T>
    where I: IntoIterator<Item=T>,
          R: Rng,
{
    let mut iter = iterable.into_iter();
    let mut reservoir: Vec<T> = iter.by_ref().take(amount).collect();
    // continue unless the iterator was exhausted
    if reservoir.len() == amount {
        for (i, elem) in iter.enumerate() {
            let k = rng.gen_range(0, i + 1 + amount);
            if let Some(spot) = reservoir.get_mut(k) {
                *spot = elem;
            }
        }
    }
    reservoir
}
