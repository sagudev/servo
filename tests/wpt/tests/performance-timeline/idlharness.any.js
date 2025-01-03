// META: global=window,worker,shadowrealm-in-window
// META: script=/resources/WebIDLParser.js
// META: script=/resources/idlharness.js

// https://w3c.github.io/performance-timeline/

'use strict';

idl_test(
  ['performance-timeline'],
  ['hr-time', 'dom'],
  async idl_array => {
    if (self.GLOBAL.isShadowRealm()) {
      return;
    }

    idl_array.add_objects({
      Performance: ['performance'],
      PerformanceObserver: ['observer'],
      PerformanceObserverEntryList: ['entryList'],
    });

    self.entryList = await new Promise((resolve, reject) => {
      self.observer = new PerformanceObserver(resolve);
      observer.observe({ entryTypes: ['mark'] });
      performance.mark('test');
    });
  }
);
