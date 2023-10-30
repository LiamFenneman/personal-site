---
name: Personal Site
description: >-
  This website you are viewing now. Made using Rust, Axum, Askama, htmx, and
  Tailwind.
created_at: 2023-10-30
updated_at: 2023-10-31
links:
  GitHub: https://github.com/LiamFenneman/personal-site
---

## Goals

> ### 1. Project showcase & documentation
>
> _A place to showcase some of the projects that I have developed and
> provide documentation about each project._

> ### 2. Publicly-available resume / CV
>
> _Provide an always-up-to-date copy of my resume / CV for anyone who wants to
> view it._

> ### 3. Develop a production application with RAAHT stack
>
> The Rust + Axum + Askama + htmx + Tailwind stack is something I have
> wanted to use in a project for a while and this project is a suitable
> candidate.

## Technology Used

- Rust: _great language for making fast and secure software._
- Axum: _arguably the best back-end framework for Rust._
- Askama: _popular templating engine that works with Axum._
- htmx: _front-end "framework" that works well in a back-end focused
  application._
- Tailwind: _simple CSS utilities so I don't have to write CSS._
- Markdown: _compact syntax for writing large text-based documents._

I first saw this stack in the "Back to the server with Rust, Axum, and htmx"
article by Joey McKenzie [1]. This article was used to create a base for this
project.

This project is using htmx since I have recently been reading the "Hypermedia
Systems" book by the developers of htmx [2]. The goals of htmx means that the
focus of the project can be to build the back-end system rather than focusing
effort using some other front-end framework. By using htmx I am not replacing
HTML with a new syntax but rather extending the functionality of HTML.

I am using Markdown to write the long-form documents since Markdown syntax sits
nicely in the gap between plain-text and HTML. Markdown can be easily converted
into HTML which allows for it to be injected into a Askama template and returned
to the user.

## Design Guidelines

> ### 1. Colour Scheme
>
> This project uses gruvbox for the colour palette since it provides a good base
> for nice colours with good contrast [3]. This colour palette also has both
> dark and light colour schemes that are closely related. By not using a custom
> colour palette I can focus on making the website work functionally and still
> have nice colours.

> ### 2. Whitespace & Layout
>
> The project is layout is similar to a blog and in many ways acts like a blog.
> This means that the appropriate layout is a single column with a fixed width
> of `768px` since this makes reading easier.

> ### 3. Responsive Design
>
> The project is designed around the idea of designing for the most simple case
> first. This is opposed to the mobile-first design that many other website use
> and recommend. The simple case first approach means that I make sure the most
> simple case is handled first and any complex cases are built on top of the
> basic case.
>
> An example of this is the fixed width of the main content. The most simple
> case is when the screen size is large (`>1024px`) since the fixed width can be
> achieved by setting the `max-width` property. The case for smaller devices is
> a bit more complex since I need to decide how the content should be compacted.
> In this case I apply a padding to the x-axis and allow the content to be full
> width.

## References

1. <https://joeymckenzie.tech/blog/templates-with-rust-axum-htmx-askama/>
1. <https://hypermedia.systems/>
1. <https://github.com/morhetz/gruvbox>
