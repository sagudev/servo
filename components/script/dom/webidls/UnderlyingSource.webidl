/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

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