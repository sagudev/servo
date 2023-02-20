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

// ReadableStreamDefaultReader.webidl

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
  Promise<ReadableStreamDefaultReadResult> read();

  [Throws]
  undefined releaseLock();
};
ReadableStreamDefaultReader includes ReadableStreamGenericReader;


dictionary ReadableStreamDefaultReadResult {
 any value;
 boolean done;
};

// ReadableStreamDefaultController.webidl

[Exposed=(Window,Worker,Worklet)]
interface ReadableStreamDefaultController {
  readonly attribute unrestricted double? desiredSize;

  [Throws]
  undefined close();

  [Throws]
  undefined enqueue(optional any chunk);

  [Throws]
  undefined error(optional any e);
};

// QueuingStrategy.webidl

dictionary QueuingStrategy {
  unrestricted double highWaterMark;
  QueuingStrategySize size;
};

callback QueuingStrategySize = unrestricted double (optional any chunk);

dictionary QueuingStrategyInit {
  required unrestricted double highWaterMark;
};


[Exposed=(Window,Worker,Worklet)]
interface CountQueuingStrategy {
  constructor(QueuingStrategyInit init);

  readonly attribute unrestricted double highWaterMark;

  // This is currently inlined, but will need to be implemented
  // See Bug 1734239
  //
  // readonly attribute Function size;
};
