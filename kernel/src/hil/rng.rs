//! Interfaces for accessing a random number generator.
//!
//! A random number generator produces a stream of random numbers,
//! either from hardware or based on an initial seed. The
//! [RNG](trait.RNG.html) trait provides a simple, implementation
//! agnostic interface for getting new random values.
//!
//! _Randomness_: Random numbers generated by this trait MUST pass
//! standard randomness tests, such as A. Rukhin, J. Soto,
//! J. Nechvatal, M. Smid, E. Barker, S. Leigh, M. Levenson,
//! M. Vangel, D. Banks, A. Heckert, J. Dray, and S. Vo. A statistical
//! test suite for random and pseudorandom number generators for
//! cryptographic applications. Technical report, NIST, 2010. It is
//! acceptable for implementations to rely on prior verification of
//! the algorithm being used. For example, if the implementation
//! chooses to use a Fishman and Moore Linear Congruence Generator
//! (LCG) with the parameters specified in the NIST report above, it
//! does not need to re-run the tests.
//!
//! Entropy: This trait does not promise high-entropy random numbers,
//! although it MAY generate them. Implementations of this interface
//! MAY generate random numbers using techniques other than true
//! random number generation (through entropy) or cryptographically
//! secure pseudorandom number generation. Other traits, described
//! elsewhere, provide random numbers with entropy guarantees. This
//! trait MUST NOT be used for randomness needed for security or
//! cryptography. If high-entropy randomness is needed, the `Entropy`
//! trait should be used instead.
//!
//! The interface is designed to work well with random number
//! generators that may not have values ready immediately. This is
//! important when generating numbers from a low-bandwidth hardware
//! random number generator or when the RNG is virtualized among many
//! consumers.
//!
//! Random numbers are yielded to the [Client](trait.Client.html) as
//! an `Iterator` which only terminates when no more numbers are
//! currently available. Clients can request more randmoness if needed
//! and will be called again when more is available.
//!
//! # Example
//!
//! The following example is a simple capsule that prints out a random number
//! once a second using the `Alarm` and `RNG` traits.
//!
//! ```
//! use kernel::hil;
//! use kernel::hil::time::Frequency;
//! use kernel::ReturnCode;
//!
//! struct RngTest<'a, A: 'a + hil::time::Alarm> {
//!     rng: &'a hil::rng::Rng,
//!     alarm: &'a A
//! }
//!
//! impl<'a, A: hil::time::Alarm> RngTest<'a, A> {
//!     pub fn initialize(&self) {
//!         let interval = 1 * <A::Frequency>::frequency();
//!         let tics = self.alarm.now().wrapping_add(interval);
//!         self.alarm.set_alarm(tics);
//!     }
//! }
//!
//! impl<'a, A: hil::time::Alarm> hil::time::Client for RngTest<'a, A> {
//!     fn fired(&self) {
//!         self.rng.get();
//!     }
//! }
//!
//! impl<'a, A: hil::time::Alarm> hil::rng::Client for RngTest<'a, A> {
//!     fn randomness_available(&self,
//!                             randomness: &mut Iterator<Item = u32>,
//!                             error: ReturnCode) -> hil::rng::Continue {
//!         match randomness.next() {
//!             Some(random) => {
//!                 println!("Rand {}", random);
//!                 let interval = 1 * <A::Frequency>::frequency();
//!                 let tics = self.alarm.now().wrapping_add(interval);
//!                 self.alarm.set_alarm(tics);
//!                 hil::rng::Continue::Done
//!             },
//!             None => hil::rng::Continue::More
//!         }
//!     }
//! }
//! ```

use returncode::ReturnCode;
/// Denotes whether the [Client](trait.Client.html) wants to be notified when
/// `More` randomness is available or if they are `Done`
#[derive(Debug, Eq, PartialEq)]
pub enum Continue {
    /// More randomness is required.
    More,
    /// No more randomness required.
    Done,
}

/// Generic interface for a 32-bit random number generator.
///
/// Implementors should assume the client implements the
/// [Client](trait.Client.html) trait.
pub trait Rng<'a> {
    /// Initiate the aquisition of new random number generation.
    ///
    /// There are three valid return values:
    ///   - SUCCESS: a `randomness_available` callback will be called in
    ///     the future when randomness is available.
    ///   - FAIL: a `randomness_available` callback will not be called in
    ///     the future, because random numbers cannot be generated. This
    ///     is a general failure condition.
    ///   - EOFF: a `randomness_available` callback will not be called in
    ///     the future, because the random number generator is off/not
    ///     powered.
    fn get(&self) -> ReturnCode;

    /// Cancel acquisition of random numbers.
    ///
    /// There are three valid return values:
    ///   - SUCCESS: an outstanding request from `get` has been cancelled,
    ///     or there was no oustanding request. No `randomness_available`
    ///     callback will be issued.
    ///   - FAIL: There will be a randomness_available callback, which
    ///     may or may not return an error code.
    fn cancel(&self) -> ReturnCode;
    fn set_client(&'a self, &'a Client);
}

/// An [Rng](trait.Rng.html) client
///
/// Clients of an [Rng](trait.Rng.html) must implement this trait.
pub trait Client {
    /// Called by the (RNG)[trait.RNG.html] when there are one or more random
    /// numbers available
    ///
    /// `randomness` in an `Iterator` of available random numbers. The amount of
    /// randomness available may increase if `randomness` is not consumed
    /// quickly so clients should not rely on iterator termination to finish
    /// consuming random numbers.
    ///
    /// The client returns either `Continue::More` if the iterator did not have
    /// enough random values and the client would like to be called again when
    /// more is available, or `Continue::Done`.
    ///
    /// If randoness_available is triggered after a call to cancel()
    /// then error MUST be ECANCEL and randomness MAY contain
    /// random bits.
    fn randomness_available(
        &self,
        randomness: &mut Iterator<Item = u32>,
        error: ReturnCode,
    ) -> Continue;
}

/// Generic interface for a synchronous 32-bit random number
/// generator.

pub trait Random<'a> {
    /// Initialize/reseed the random number generator from an
    /// internal source. This initialization MAY be deterministic
    /// (e.g., based on an EUI-64) or MAY be random (e.g., based on an
    /// underlying hardware entropy source); an implementation SHOULD
    /// make reseeding random.
    fn initialize(&'a self);

    /// Reseed the random number generator with a specific
    /// seed. Useful for deterministic tests.
    fn reseed(&self, seed: u32);

    /// Generate a 32-bit random number.
    fn random(&self) -> u32;
}
