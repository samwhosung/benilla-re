(function () {
  'use strict';
  var root = document.documentElement.getAttribute('data-root') || '';
  var index = null, loading = null;

  function load() {
    if (!loading) {
      loading = fetch(root + 'index.json').then(function (r) { return r.json(); }).then(function (j) {
        index = j;
        return j;
      });
    }
    return loading;
  }

  // Six digits at least, never cut.
  function digits(n) { var s = n.toString(16); return s.length < 6 ? ('00000' + s).slice(-6) : s; }
  function hex(n) { return '0x' + digits(n); }
  function page(row) { return 'a/' + digits(row[0]) + '.html'; }

  // The row with start <= a < start + size: the nearest start at or below a, else an earlier
  // row that reaches it.
  function holding(a) {
    var rows = index.a, lo = 0, hi = rows.length - 1, best = -1;
    while (lo <= hi) {
      var mid = (lo + hi) >> 1;
      if (rows[mid][0] <= a) { best = mid; lo = mid + 1; } else { hi = mid - 1; }
    }
    for (var i = best, n = 0; i >= 0 && n < 8; i--, n++) {
      if (a < rows[i][0] + Math.max(rows[i][1], 1)) { return rows[i]; }
    }
    return null;
  }

  function target(a) {
    var row = holding(a);
    if (!row) { return null; }
    return root + page(row) + (row[0] === a ? '' : '#' + hex(a));
  }

  function addressOf(text) {
    var m = /^\s*0x([0-9a-f]{1,8})\s*$/i.exec(text);
    return m ? parseInt(m[1], 16) : null;
  }

  // ?0x5e6850 goes to the row that holds the address; the 0x is optional and anything after
  // an & is ignored.
  var raw = '';
  try { raw = decodeURIComponent(location.search.slice(1)).split('&')[0].trim(); } catch (e) { raw = ''; }
  if (raw !== '') {
    var parsed = /^(?:0x)?([0-9a-f]{1,8})$/i.exec(raw);
    var note = document.getElementById('resolve');
    if (!parsed) {
      if (note) {
        note.hidden = false;
        note.textContent = raw.slice(0, 40) + ' is not an address.';
      }
    } else {
      var query = parseInt(parsed[1], 16);
      load().then(function () {
        var to = target(query);
        if (to) {
          location.replace(to);
        } else if (note) {
          note.hidden = false;
          note.textContent = 'No row holds ' + hex(query) + '.';
        }
      });
    }
  }

  // An address page opened at an instruction says where it is.
  var at = document.getElementById('at');
  if (at && /^#0x[0-9a-f]+$/i.test(location.hash)) {
    var code = document.querySelector('dd code');
    var start = code ? parseInt(code.textContent.slice(2), 16) : null;
    var a = parseInt(location.hash.slice(3), 16);
    if (start !== null && a >= start) {
      at.hidden = false;
      var sub = document.querySelector('.sub');
      var unit = sub && sub.textContent.indexOf('global') === 0 ? 'byte' : 'instruction';
      at.textContent = hex(a) + ' is the ' + unit + ' at +0x' + (a - start).toString(16) + ' of this row.';
    }
  }

  // Search over names and addresses.
  var box = document.getElementById('q'), hits = document.getElementById('hits');
  if (!box || !hits) { return; }
  box.addEventListener('focus', load);
  box.addEventListener('input', function () {
    var q = box.value.trim().toLowerCase();
    hits.textContent = '';
    if (q.length < 2) { return; }
    load().then(function () {
      if (box.value.trim().toLowerCase() !== q) { return; }
      var out = [], a = addressOf(q);
      if (a !== null) {
        var to = target(a);
        if (to) { out.push([hex(a) + ' (the row that holds it)', to]); }
      }
      var prefix = [], inside = [];
      function scan(label, href) {
        var l = label.toLowerCase(), k = l.indexOf(q);
        if (k === 0) { prefix.push([label, href]); } else if (k > 0) { inside.push([label, href]); }
      }
      index.o.forEach(function (o) { scan(o[0] + ' (' + o[2] + ')', root + o[1]); });
      index.a.forEach(function (r) {
        if (r[2]) { scan(r[2] + ' ' + hex(r[0]), root + page(r)); } else if (q.charAt(0) === '0') { scan(hex(r[0]), root + page(r)); }
      });
      out = out.concat(prefix, inside).slice(0, 30);
      out.forEach(function (h) {
        var li = document.createElement('li'), link = document.createElement('a');
        link.href = h[1];
        link.textContent = h[0];
        li.appendChild(link);
        hits.appendChild(li);
      });
    });
  });
  document.addEventListener('click', function (e) {
    if (!hits.contains(e.target) && e.target !== box) { hits.textContent = ''; }
  });
})();
