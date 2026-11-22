const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../public/browser-audio-session.js'), 'utf8');
function boot(navigator, native = false) {
  const listeners = {}, audio = {addEventListener(name, fn) {listeners[name] = fn;}};
  vm.runInNewContext(source, {window: {vibefyNativePlayback: native}, navigator,
    document: {getElementById: () => audio}});
  return listeners;
}
test('a direct audio play requests the music session without autoplay or visibility hooks', () => {
  const session = {type: 'auto'}, listeners = boot({audioSession: session});
  assert.equal(session.type, 'auto');
  assert.deepEqual(Object.keys(listeners), ['play']);
  listeners.play(); assert.equal(session.type, 'playback');
});
test('unavailable or rejected session APIs cannot break direct audio', () => {
  assert.doesNotThrow(() => boot({}).play());
  const session = {}; Object.defineProperty(session, 'type', {set() {throw Error('unsupported');}});
  assert.doesNotThrow(() => boot({audioSession: session}).play());
});
test('native audio keeps its own session', () => {
  assert.deepEqual(Object.keys(boot({audioSession: {type: 'auto'}}, true)), []);
});
