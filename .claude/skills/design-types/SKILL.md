---
name: design-types
description: Design Rust types for an upstream's data and for what the apps receive, and the conversion between them. Use when adding or changing any struct, enum, trait or error type in this repository, when porting a route's types, or when reviewing whether a type is exact enough.
---

# Design types

Follow [the type layout](../../../docs/types.md) for where schemas, conversions and errors live.

The maintainer's primary objective for this project is to use Rust for what it is good at: the type system and performance. A type that permits a state the data can never be in is a defect here, even when every test passes. Types and their documentation are reviewed before any body is written.

## The reference decides what is accepted

Accept every input the reference accepts and produce the same output for it (rule P0). A more exact type is about how an accepted input is represented. It never rejects something the reference accepts, adds an error the reference does not have, or changes what is sent. When exactness and the reference conflict, follow the reference and make the type fit it.

## Read before designing

In the reference (`tools/reference-path`):

1. The schema for the upstream's response. It lists the fields the reference reads and which are optional, often with a comment saying why.
2. The conversion function, to see which fields are used and how.
3. The fixtures and the tests. A field is optional only if the schema says so or a fixture leaves it out.

Model the fields the reference's schema models. Serde ignores the rest, and each extra field is another way for an upstream change to break us.

## Questions to ask of every field

- **Can the upstream leave it out, or send null?** Only then is it an `Option`. A derived `Option` accepts both; if the reference accepts only one, say so in the documentation and test it. Use `#[serde(default)]` only where the reference substitutes the same default.
- **Is it one of a fixed set of values?** Then it is an enum. Add a catch-all variant only if the reference accepts values it does not know.
- **Does it identify something from an open set?** A posting id is a newtype, not a `String` or `u32`. An identifier from a closed set, such as a campus, or anything the reference checks against a fixed table, such as its cafés, is an enum.
- **Does it have a format or a unit?** A URL is a `Url`, unless the reference passes it on unchanged: `Url` normalizes what it parses, so check it with `Url` and keep the original string. A time is a time type with its zone made explicit, not a `String`. A duration says its unit in its type or its name.
- **Can the upstream put something else in its place?** WordPress embeds an error object where an author would be. That is an enum with one variant per case. Prefer a tag the upstream sends to choose the variant; use `#[serde(untagged)]` only when there is none.
- **Do two fields only make sense together?** Then they are one struct, and an `Option` of that struct if the pair can be absent.
- **Do two fields exclude each other?** Then they are one enum, not two `Option`s or a `bool` beside an `Option`.
- **Is there a rule about the value** (not empty, within a range)? Then the field is private and a constructor checks the rule, deserializing through it with `#[serde(try_from = "...")]`. Reject a value only if the reference rejects it.

## Struct or enum

A struct says all of these exist together. An enum says exactly one of these is the case. If you find yourself writing "when `x` is set, `y` is ignored" in documentation, the type should be an enum.

## Traits

Add a trait when more than one type provides the same capability, such as a cache backend or a way of fetching from a site. Do not add one for a single implementation. Where each implementor has exactly one choice of a type, make it an associated type.

## The conversion

- `From` when it cannot fail, `TryFrom` with its own error enum when it can fail and needs nothing else, and a named function when it needs another input such as the campus date (rule T2).
- The error enum has one variant per way the upstream's data can be unusable that the reference also treats as a failure, each carrying what a person needs to find the bad record.
- No `#[non_exhaustive]` inside the workspace: a new error case should break the `match` that turns errors into responses.
- No I/O, clock or environment. A step that loses information, such as turning HTML into text, is its own function with its own tests.
- Take the upstream value by value and move its parts. Do not clone to convert.

## Documentation is part of the type

Write it with the type. Every public type, field and variant says what it represents, not what its name already says. In addition:

- an upstream type says which request produces it and links the reference's schema;
- a field says when it can be absent and what that means;
- a type with a rule states the rule;
- a function that can fail has an `# Errors` section.

## What to show for review

In the draft pull request that holds the types and signatures, include a table: the reference's field, the Rust field and type, and the reason for any difference (why not an `Option`, why an enum). The reviewer checks the table against the reference.

## An illustration

The upstream here is invented, so that nobody mistakes it for a port of a real one. It is the whole of a small crate that compiles under `#![deny(missing_docs)]` and whose tests pass; the tests are left out. It shows the questions above being answered, and it is not an approved pattern. Once routes have been approved, follow them and not this.

```rust
use serde::{Deserialize, Serialize};

/// One stop, as the shuttle service's timetable API sends it.
///
/// Only the fields the conversion reads are modelled, and serde ignores the
/// rest.
#[derive(Debug, Deserialize)]
pub struct Stop {
	/// The service's identifier for the stop.
	pub id: StopId,
	/// Which campus the stop is on.
	pub campus: Campus,
	/// Whether shuttles are calling at the stop, and what follows from that.
	pub service: Service,
}

/// A stop's identifier.
///
/// The service adds and removes stops, so this is an opaque value and not an
/// enum. It is a type of its own so that it cannot be passed where some other
/// string is expected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StopId(String);

/// A campus the shuttle serves. There are exactly two, so this is an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Campus {
	/// The north campus.
	North,
	/// The south campus.
	South,
}

/// Whether shuttles are calling at a stop.
///
/// A running stop has a next departure and a suspended stop has a reason,
/// never both and never neither. That is one of two cases, so this is an enum
/// and not a struct with two optional fields.
#[derive(Debug, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Service {
	/// Shuttles are calling.
	Running {
		/// Whole minutes until the next shuttle leaves.
		next_in_minutes: u16,
	},
	/// Shuttles are not calling.
	Suspended {
		/// Why, in the service's words.
		reason: String,
	},
}

/// The next departure from a stop, as the apps receive it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Departure {
	/// The stop.
	pub stop: StopId,
	/// The campus the stop is on.
	pub campus: Campus,
	/// Whole minutes until the shuttle leaves.
	pub minutes: u16,
}

/// Why a [`Stop`] has no [`Departure`].
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum StopError {
	/// Shuttles are not calling at the stop, so there is no departure to show.
	#[error("stop {stop:?} is suspended: {reason}")]
	Suspended {
		/// The stop.
		stop: StopId,
		/// The service's reason.
		reason: String,
	},
}

impl TryFrom<Stop> for Departure {
	type Error = StopError;

	/// Converts a running stop into its next departure.
	///
	/// # Errors
	///
	/// Returns [`StopError::Suspended`] when shuttles are not calling at the
	/// stop.
	fn try_from(stop: Stop) -> Result<Self, Self::Error> {
		match stop.service {
			Service::Running { next_in_minutes } => {
				Ok(Self { stop: stop.id, campus: stop.campus, minutes: next_in_minutes })
			}
			Service::Suspended { reason } => Err(StopError::Suspended { stop: stop.id, reason }),
		}
	}
}
```

An upstream type derives `Deserialize` and not `Serialize`. Other derives, such as `Debug`, are fine.

## What not to write

- One struct deserialized from an upstream and serialized to the apps.
- Every field an `Option<String>` to be safe.
- `serde_json::Value` for something the reference gives a shape to.
- A `String` error, or one error variant for every failure.
- A type loosened so that a failing fixture passes. The fixture is telling you about a case to name.
