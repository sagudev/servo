/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// Source: Streams Standard (https://streams.spec.whatwg.org/)
// and https://hg.mozilla.org/mozilla-central/file/b25a4258575d1136d75e270acaadd99101414fe9/dom/webidl/ReadableStream.webidl

[Exposed=(Window,Worker,Worklet)]
interface _ReadableStream {
  [Throws]
  constructor(optional object underlyingSource, optional QueuingStrategy strategy = {});

  readonly attribute boolean locked;

  [Throws]
  Promise<undefined> cancel(optional any reason);
  [Throws]
  ReadableStreamReader getReader(optional ReadableStreamGetReaderOptions options = {});
  //ReadableStream pipeThrough(ReadableWritablePair transform, optional StreamPipeOptions options = {});
  //Promise<undefined> pipeTo(WritableStream destination, optional StreamPipeOptions options = {});
  //sequence<ReadableStream> tee();

  //async iterable<any>(optional ReadableStreamIteratorOptions options = {});
};

// This typedef should actually be the union of ReadableStreamDefaultReader
// and ReadableStreamBYOBReader. However, we've not implmented the latter
// yet, and so for now the typedef is subset.
typedef ReadableStreamDefaultReader ReadableStreamReader;
//typedef (ReadableStreamDefaultReader or ReadableStreamBYOBReader) ReadableStreamReader;

enum ReadableStreamReaderMode { "byob" };

dictionary ReadableStreamGetReaderOptions {
  ReadableStreamReaderMode mode;
};

dictionary UnderlyingSource {
  UnderlyingSourceStartCallback start;
  UnderlyingSourcePullCallback pull;
  UnderlyingSourceCancelCallback cancel;
  ReadableStreamType type;
  [EnforceRange] unsigned long long autoAllocateChunkSize;
};

//typedef (ReadableStreamDefaultController or ReadableByteStreamController) ReadableStreamController;
typedef ReadableStreamDefaultController ReadableStreamController;

callback UnderlyingSourceStartCallback = any (ReadableStreamController controller);
callback UnderlyingSourcePullCallback = Promise<undefined> (ReadableStreamController controller);
callback UnderlyingSourceCancelCallback = Promise<undefined> (optional any reason);

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

/*[Exposed=(Window,Worker,Worklet)]
interface ReadableStreamBYOBReader {
  constructor(ReadableStream stream);

  Promise<ReadableStreamReadResult> read(ArrayBufferView view);
  undefined releaseLock();
};
ReadableStreamBYOBReader includes ReadableStreamGenericReader;
*/
[Exposed=(Window,Worker,Worklet)]
interface ReadableStreamDefaultController {
  readonly attribute unrestricted double? desiredSize;

  undefined close();
  undefined enqueue(optional any chunk);
  undefined error(optional any e);
};
/*
[Exposed=(Window,Worker,Worklet)]
interface ReadableByteStreamController {
  readonly attribute ReadableStreamBYOBRequest? byobRequest;
  readonly attribute unrestricted double? desiredSize;

  undefined close();
  undefined enqueue(ArrayBufferView chunk);
  undefined error(optional any e);
};


[Exposed=(Window,Worker,Worklet)]
interface ReadableStreamBYOBRequest {
  readonly attribute ArrayBufferView? view;

  undefined respond([EnforceRange] unsigned long long bytesWritten);
  undefined respondWithNewView(ArrayBufferView view);
};
*/

dictionary QueuingStrategy {
  unrestricted double highWaterMark;
  QueuingStrategySize size;
};

callback QueuingStrategySize = unrestricted double (any chunk);

dictionary QueuingStrategyInit {
  required unrestricted double highWaterMark;
};

[Exposed=(Window,Worker,Worklet)]
interface ByteLengthQueuingStrategy {
  constructor(QueuingStrategyInit init);

  readonly attribute unrestricted double highWaterMark;
  //readonly attribute Function size;
};

[Exposed=(Window,Worker,Worklet)]
interface CountQueuingStrategy {
  constructor(QueuingStrategyInit init);

  readonly attribute unrestricted double highWaterMark;
  readonly attribute Function size;
};