# Legal: scope and limits

A short, honest assessment. Not legal advice.

## What this is

Software to drive cutting machines you own, with your own code, for your own
use. We read the message format of Design Space to learn how the machines
communicate (interoperability). We do not run Design Space to drive anything,
and we do not copy Cricut's code into this repository.

## Where it gets sensitive

1. **Cricut's EULA forbids reverse engineering.** That is a contractual term. In
   the EU there is mandatory law against it: decompiling for interoperability is
   allowed under conditions (Software Directive 2009/24/EC, art. 6, and the
   right to study how a program works, art. 5). That exception exists for
   exactly this: making your own program work together with existing hardware.

2. **Circumventing the encryption.** The Provo Craft lawsuits of 2010-2011
   (Make The Cut, Sure Cuts A Lot) were about copying code and about
   circumventing the USB encryption. The projects that work today put Cricut's
   keys literally in their source; their own NOTICE files warn about the legal
   exposure. That is the point where a technical protection measure is
   circumvented, and it is the line this project stays behind.

## Scope

This repository contains the protocol stack, including an implementation of the
session cipher, for interoperability with hardware you own. Hard limits that
stand:

- **No keys and no code taken from Design Space** in the repository. The
  session key, the unwrap seed, and the materials text are loaded at runtime
  from outside version control. They are not copied into this tree. That is the
  difference between studying how it works (allowed for interoperability) and
  circumventing a protection measure with copied keys (the line).

Use this only for machines you own.
