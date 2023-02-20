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
