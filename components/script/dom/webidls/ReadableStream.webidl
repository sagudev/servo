/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

[Exposed=(Window,Worker,Worklet)]
interface ReadableStream {
  [Throws]
  constructor(optional object underlyingSource, optional QueuingStrategy strategy = {});

  readonly attribute boolean locked;

  [Throws]
  Promise<undefined> cancel(optional any reason);

  [Throws]
  ReadableStreamReader getReader(optional ReadableStreamGetReaderOptions options = {});

  // Bug 1734243
  // ReadableStream pipeThrough(ReadableWritablePair transform, optional StreamPipeOptions options = {});

  // Bug 1734241
  // Promise<undefined> pipeTo(WritableStream destination, optional StreamPipeOptions options = {});
  // sequence<ReadableStream> tee();

  // Bug 1734244
  // async iterable<any>(optional ReadableStreamIteratorOptions options = {});
};

enum ReadableStreamReaderMode { "byob" };

dictionary ReadableStreamGetReaderOptions {
  ReadableStreamReaderMode mode;
};

[GenerateInit]
dictionary UnderlyingSource {
  UnderlyingSourceStartCallback start;
  UnderlyingSourcePullCallback pull;
  UnderlyingSourceCancelCallback cancel;
  ReadableStreamType type;
  [EnforceRange] unsigned long long autoAllocateChunkSize;
};

// Until ReadableByteStreamController is implemented, this typedef is only a subset.
typedef ReadableStreamDefaultController ReadableStreamController;

callback UnderlyingSourceStartCallback = any (ReadableStreamController controller);
callback UnderlyingSourcePullCallback = Promise<undefined> (ReadableStreamController controller);
callback UnderlyingSourceCancelCallback = Promise<undefined> (optional any reason);