---
name: design-types
description: Design Rust types for an upstream's data and for what the apps receive, and the conversion between them. Use when adding or changing any struct, enum, trait or error type in this repository, when porting a route's types, or when reviewing whether a type is exact enough.
---

# Design types

The maintainer's primary objective for this project is to use Rust for what it is good at: the type system and performance. A type that permits a state the data can never be in is a defect here, even when every test passes. Types and their documentation are reviewed before any body is written.

## Read before designing

In the reference (`tools/reference-path`):

1. The schema for the upstream's response. It lists the fields the reference reads and which are optional, often with a comment saying why.
2. The conversion function, to see which fields are used and how.
3. The fixtures and the tests. A field is optional only if the schema says so or a fixture leaves it out.

Model the fields the reference's schema models. Serde ignores the rest, and each extra field is another way for an upstream change to break us.

## Questions to ask of every field

- **Can the upstream leave it out, or send null?** Only then is it an `Option`. If the reference treats absent and null differently, so do we.
- **Is it one of a fixed set of values?** Then it is an enum. Add a catch-all variant only if the reference accepts values it does not know.
- **Does it identify something?** A café id, a campus, a feed name is a newtype, not a `String` or `u32`.
- **Does it have a format or a unit?** A URL is a `Url`. A time is a time type with its zone made explicit, not a `String`. A duration says its unit in its type or its name.
- **Can the upstream put something else in its place?** WordPress embeds an error object where an author would be. That is an enum with one variant per case.
- **Do two fields only make sense together, or exclude each other?** Then they are one enum, not two `Option`s or a `bool` beside an `Option`.
- **Is there a rule about the value** (not empty, within a range)? Then the field is private and a constructor checks the rule, or the conversion rejects the value with a named error.

## Struct or enum

A struct says all of these exist together. An enum says exactly one of these is the case. If you find yourself writing "when `x` is set, `y` is ignored" in documentation, the type should be an enum.

## Traits

Add a trait when more than one type provides the same capability, such as a cache backend or a way of fetching from a site. Do not add one for a single implementation. Where each implementor has exactly one choice of a type, make it an associated type.

## The conversion

- `impl TryFrom<Upstream> for Ccc`, with its own error enum. One variant per way the upstream's data can be unusable, each carrying what a person needs to find the bad record.
- `#[non_exhaustive]` on the error enum.
- No I/O, clock or environment. A step that loses information, such as turning HTML into text, is its own function with its own tests.
- Take the upstream value by value and move its parts. Do not clone to convert.

## Documentation is part of the type

Write it with the type. Every public type, field and variant says what it represents, not what its name already says. In addition:

- an upstream type says which request produces it and links the reference's schema;
- a field says when it can be absent and what that means;
- a type with a rule states the rule;
- a function that can fail has an `# Errors` section.

## What to show for review

In the types-and-signatures pull request, include a table: the reference's field, the Rust field and type, and the reason for any difference (why not an `Option`, why an enum). The reviewer checks the table against the reference.

## An illustration

This compiles and is tested, but it is an illustration of the questions above, not an approved pattern. Once routes have been approved, follow them and not this.

```rust
/// One post, as a WordPress site's REST API sends it when asked to embed
/// related records (`/wp-json/wp/v2/posts?_embed`).
///
/// Only the fields the conversion reads are modelled, and serde ignores the
/// rest. The shape follows the reference's schema for the same response.
#[derive(Debug, Deserialize)]
pub struct Post {
	/// The post's permanent address. WordPress always sends it.
	pub link: Url,
	/// The title, as HTML. WordPress always sends it, and it may be empty.
	pub title: Rendered,
	/// The post's authors, present because the request asked for `_embed`.
	#[serde(rename = "_embedded", default)]
	pub embedded: Embedded,
}

/// An author embedded in a post, or what WordPress sends in an author's place.
///
/// A site that has turned off its users endpoint embeds an error object where
/// each author would be. That is one of two cases, so this is an enum and not
/// a struct of optional fields.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum EmbeddedAuthor {
	/// The author's record.
	Known {
		/// The author's display name.
		name: String,
	},
	/// The site refused to say who the author is.
	Withheld {
		/// WordPress's error code, such as `rest_user_cannot_view`.
		code: String,
	},
}

/// A news item, as the apps receive it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct FeedItem {
	/// The headline, as plain text. Never empty.
	pub title: String,
	/// Where the full story is.
	pub link: Url,
	/// The authors' names. Empty when the site withholds them.
	pub authors: Vec<String>,
}

/// Why a [`Post`] could not become a [`FeedItem`].
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PostError {
	/// The post's title was empty, so there is nothing to show in a list.
	#[error("the post at {link} has an empty title")]
	EmptyTitle {
		/// The post's address, to find it by.
		link: Url,
	},
}

impl TryFrom<Post> for FeedItem {
	type Error = PostError;

	/// Converts a post, keeping the names of the authors the site disclosed.
	///
	/// # Errors
	///
	/// Returns [`PostError::EmptyTitle`] when the title is empty.
	fn try_from(post: Post) -> Result<Self, Self::Error> {
		if post.title.rendered.is_empty() {
			return Err(PostError::EmptyTitle { link: post.link });
		}
		let authors = post
			.embedded
			.author
			.into_iter()
			.filter_map(|author| match author {
				EmbeddedAuthor::Known { name } => Some(name),
				EmbeddedAuthor::Withheld { .. } => None,
			})
			.collect();
		Ok(Self { title: post.title.rendered, link: post.link, authors })
	}
}
```

## What not to write

- One struct deserialized from an upstream and serialized to the apps.
- Every field an `Option<String>` to be safe.
- `serde_json::Value` for something the reference gives a shape to.
- A `String` error, or one error variant for every failure.
- A type loosened so that a failing fixture passes. The fixture is telling you about a case to name.
