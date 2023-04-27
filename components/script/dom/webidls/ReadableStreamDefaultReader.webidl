/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// This typedef should actually be the union of ReadableStreamDefaultReader
// and ReadableStreamBYOBReader. However, we've not implmented the latter
// yet, and so for now the typedef is subset.
typedef ReadableStreamDefaultReader ReadableStreamReader;


enum ReadableStreamType { "bytes" };

interface mixin ReadableStreamGenericReader {
  readonly attribute Promise<undefined> closed;

  [Throws]
  Promise<undefined> cancel(optional any reason);
};

[Exposed=(Window,Worker,Worklet)]
interface ReadableStreamDefaultReader {
  [Throws]
  constructor(ReadableStream stream);

  [Throws]
  Promise<ReadableStreamReadResult> read();

  [Throws]
  undefined releaseLock();
};
ReadableStreamDefaultReader includes ReadableStreamGenericReader;


dictionary ReadableStreamReadResult {
 any value;
 boolean done;
};
