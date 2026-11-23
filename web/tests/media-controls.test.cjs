const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const read = name => fs.readFileSync(path.join(__dirname, '../public', name), 'utf8');
const playerSource = read('player.js');

test('an iPhone hands off the full YouTube queue in shuffle order and preserves the chosen starting track', () => {
    const code = playerSource.slice(playerSource.indexOf('function embeddedQueue('),
        playerSource.indexOf('function refreshEmbeddedQueue()'));
    const queue = ['abcdefghijk','11aaBBccDDe','zyxwvutsrqp'].map(id => ({id}));
    const context = vm.createContext({
        queue, index: 1, shuffle: true, repeat: true, navigator: {userAgent: 'iPhone'}, window: {},
        Math: {floor: Math.floor, random: () => 0}
    });
    vm.runInContext(code, context);
    const shuffled = context.embeddedQueue();
    assert.deepEqual(Array.from(shuffled.ids), [queue[1].id, queue[2].id, queue[0].id]);
    assert.deepEqual(Array.from(shuffled.indices), [1, 2, 0]);
    assert.equal(shuffled.start, 0);
    assert.equal(shuffled.repeat, true);
    context.shuffle = false;
    assert.equal(context.embeddedQueue().start, 1);
    context.window.vibefyJam = {active: true};
    assert.equal(context.embeddedQueue(), undefined);
    context.window = {vibefyPreferences: {autoNext: false}};
    assert.equal(context.embeddedQueue(), undefined);
    context.window = {};
    queue.push({id: 'spotify:unresolved'});
    assert.equal(context.embeddedQueue().nextOutside, true);
    assert.equal(context.embeddedQueue().repeat, false);
    context.shuffle = true;
    assert.equal(context.embeddedQueue(), undefined);
});

test('Media Session handles relative and absolute seeking and explicitly selects both neighboring tracks', () => {
    const handlers = new Map();
    const calls = [];
    const context = {
        navigator: {mediaSession: {setActionHandler: (name, fn) => handlers.set(name, fn)}},
        media: {paused: false},
        window: {},
        shownPosition: () => 30,
        advance: (...args) => calls.push(args),
        seekTo: seconds => calls.push(seconds)
    };
    const start = playerSource.indexOf('function registerMediaSessionActions()');
    const end = playerSource.indexOf('const lyricRequests', start);
    vm.runInNewContext(playerSource.slice(start, end), context);
    handlers.get('seekforward')();
    handlers.get('seekbackward')({seekOffset: 7});
    handlers.get('seekto')({seekTime: 55});
    handlers.get('seekto')({});
    handlers.get('nexttrack')();
    handlers.get('previoustrack')();
    assert.deepEqual(calls, [40, 23, 55, [1, true], [-1, true]]);
});

test('lock-screen Previous selects the previous queue entry while in-page Previous can restart the song', () => {
    const calls = [];
    const context = {
        queue: [{id: 'first'}, {id: 'second'}], index: 1, shuffle: false, repeat: false,
        window: {}, embedded: {}, media: {currentTime: 12}, current: () => context.queue[context.index],
        loadCurrent: (...args) => calls.push(args), seekTo: value => calls.push(value)
    };
    vm.createContext(context);
    vm.runInContext(playerSource.match(/function advance\([^\n]+/)[0], context);
    context.advance(-1);
    assert.equal(context.index, 1);
    context.advance(-1, true);
    assert.equal(context.index, 0);
    assert.deepEqual(calls, [0, [false, 0, true]]);
});

test('the official player owns playlist navigation and an ended item does not advance it twice', async () => {
    const calls = [];
    const window = {vibefyPhone: true};
    const order = {ids: ['abcdefghijk','zyxwvutsrqp'], indices: [4, 2], start: 0, repeat: false};
    let position = 0, events;
    const player = {
        setVolume() {},
        setLoop: value => calls.push(['loop', value]),
        loadPlaylist: (ids, index, seconds) => calls.push(['load', ids, index, seconds]),
        getPlaylist: () => order.ids,
        getPlaylistIndex: () => position,
        nextVideo: () => calls.push('next'),
        previousVideo: () => calls.push('previous')
    };
    function Player(_element, options) {
        events = options.events;
        events.onReady({target: player});
        return player;
    }
    const document = {hidden: false, createElement: () => ({}), head: {append: () => window.onYouTubeIframeAPIReady()}};
    vm.runInNewContext(read('embedded-player.js'), {
        window, document, YT: {Player}, location: {origin: 'https://example.test'}, setTimeout() {}, clearTimeout() {}
    });
    const embed = window.vibefyEmbedded;
    embed.onEnded = () => calls.push('app-advance');
    embed.onTrack = index => calls.push(['track', index]);
    await embed.prepare();
    assert.deepEqual(calls, []);
    const loading = embed.loadQueue(order.ids[0], false, 16, false, order);
    assert.deepEqual(calls, [['loop', false], ['load', order.ids, 0, 16]]);
    await loading;
    events.onStateChange({data: 1});
    events.onStateChange({data: 0});
    position = 1;
    events.onStateChange({data: 1});
    assert.equal(embed.move(1), true);
    assert.equal(embed.move(-1), true);
    assert.deepEqual(calls, [['loop', false], ['load', order.ids, 0, 16], ['track', 4], ['track', 2], 'next', 'previous']);
});

test('seeking reaches the embedded player while stream duration is still loading', () => {
    const context = {
        current: () => ({id: 'abcdefghijk', duration: 180}),
        media: {duration: 0, currentTime: 0},
        window: {}, embedded: {active: true},
        pendingAudioSeek: null, optimisticSeek: null, updatePosition() {}
    };
    const start = playerSource.indexOf('function seekTo(');
    const end = playerSource.indexOf("for(const id of ['seek','mini-seek'])", start);
    vm.runInNewContext(playerSource.slice(start, end), context);
    context.seekTo(190);
    assert.equal(context.media.currentTime, 180);
    assert.equal(context.optimisticSeek.position, 180);
});

test('a hidden YouTube tab distinguishes explicit media actions from passive loading', async () => {
    const calls = [];
    const window = {vibefyPhone: true};
    const player = {
        setVolume() {},
        cueVideoById: item => calls.push(['cue', item.videoId]),
        loadVideoById: item => calls.push(['play', item.videoId])
    };
    const document = {
        hidden: true, createElement: () => ({}),
        head: {append: () => window.onYouTubeIframeAPIReady()}
    };
    function Player(_element, options) {
        options.events.onReady({target: player});
        return player;
    }
    vm.runInNewContext(read('embedded-player.js'), {
        window, document, YT: {Player}, location: {origin: 'https://example.test'},
        setTimeout() {}, clearTimeout() {}
    });
    await window.vibefyEmbedded.load('abcdefghijk');
    await window.vibefyEmbedded.load('zyxwvutsrqp', false, 0, true);
    await window.vibefyEmbedded.load('abcdefghijk', true, 0, true);
    assert.deepEqual(calls, [['cue', 'abcdefghijk'], ['play', 'zyxwvutsrqp'], ['cue', 'abcdefghijk']]);
});
