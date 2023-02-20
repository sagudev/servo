dictionary QueuingStrategy {
  unrestricted double highWaterMark;
  QueuingStrategySize size;
};

callback QueuingStrategySize = unrestricted double (optional any chunk);

dictionary QueuingStrategyInit {
  required unrestricted double highWaterMark;
};