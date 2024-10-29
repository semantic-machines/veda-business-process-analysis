/*

// API calls caching service worker

const watchTimeout = 60 * 1000;
const FILES = 'files';
const API = 'api';

const API_FNS = [
  '/authenticate',
  '/get_ticket_trusted',
  '/is_ticket_valid',
  '/logout',
  '/get_rights',
  '/get_rights_origin',
  '/get_membership',
  '/get_operation_state',
  '/query',
  '/stored_query',
  '/get_individual',
  '/get_individuals',
  '/remove_individual',
  '/put_individual',
  '/add_to_individual',
  '/set_in_individual',
  '/remove_from_individual',
  '/put_individuals',
  '/watch',
];

function watchChanges (CACHE) {
  if (typeof EventSource === 'undefined') return;

  const events = new EventSource('/watch');

  events.onopen = () => {
    console.log(new Date().toISOString(), 'Watching resources changes');
  };

  events.onerror = (event) => {
    console.log(new Date().toISOString(), `Failed to watch resources changes, reconnect in ${Math.floor(watchTimeout / 1000)} sec`);
    event.target.close();
    setTimeout(watchChanges, watchTimeout);
  };

  events.onmessage = (event) => {
    const change = JSON.parse(event.data);
    Object.keys(change).forEach((_path) => {
      const path = (_path === '/index.html' ? '/' : _path);
      caches.match(path).then((response) => {
        if (response && response.ok) {
          const cache_modified = response.headers.get('last-modified');
          const event_modified = change[path];
          if (cache_modified !== event_modified) {
            caches.open(CACHE).then((cache) => cache.delete(path)).then(() => {
              console.log(new Date().toISOString(), 'Cached resource deleted: ', path);
            });
          }
        }
      });
    });
  };
}
watchChanges(FILES);

// Clear cached resources
self.addEventListener('install', (event) => {
  this.skipWaiting();
  console.log('Service worker updated, clear cache');
  event.waitUntil(
    caches.keys().then((keyList) => Promise.all(keyList.map((key) => caches.delete(key)))),
  );
});

self.addEventListener('fetch', function (event) {
  const url = new URL(event.request.url);
  const pathname = url.pathname;
  const isAPI = API_FNS.indexOf(pathname) >= 0;
  const METHOD = event.request.method;
  if (METHOD !== 'GET') return;
  if (isAPI) {
    event.respondWith(handleAPI(event, API));
  } else {
    event.respondWith(handleFetch(event, FILES));
  }
});

function handleFetch (event, CACHE) {
  const path = new URL(event.request.url).pathname;
  return caches.match(path).then((cached) => cached || fetch(event.request).then((response) => {
    if (response.ok && !cached) {
      const clone = response.clone();
      caches.open(CACHE).then((cache) => {
        cache.put(event.request, clone);
      });
    }
    return response;
  }));
}

function handleAPI (event, CACHE) {
  const url = new URL(event.request.url);
  url.searchParams.delete('ticket');
  const fn = url.pathname.split('/').pop();
  switch (fn) {
  // Fetch first
  case 'get_rights':
  case 'get_rights_origin':
  case 'get_membership':
    return fetch(event.request)
      .then((response) => {
        if (response.ok) {
          const clone = response.clone();
          caches.open( CACHE ).then((cache) => {
            cache.put(url, clone);
          });
        } else if (response.status === 0 || response.status === 503) {
          return caches.match(url).then((cached) => cached || response);
        }
        return response;
      })
      // Network error
      .catch((error) => {
        return caches.match(url).then((cached) => {
          if (cached) return cached;
          throw error;
        });
      });

  // Cache first
  case 'get_individual':
    return caches.match(url).then((cached) => cached || fetch(event.request)
      .then((response) => {
        url.searchParams.delete('vsn');
        if (response.ok) {
          const clone = response.clone();
          caches.open( CACHE ).then((cache) => cache.put(url, clone));
        } else if (response.status === 0 || response.status === 503) {
          if (cached) return cached;
        }
        return response;
      }))
      // Network error
      .catch((error) => {
        url.searchParams.delete('vsn');
        return caches.match(url).then((cached) => {
          if (cached) return cached;
          throw error;
        });
      });

  // Fetch only
  case 'authenticate':
  case 'get_ticket_trusted':
  case 'is_ticket_valid':
  case 'logout':
  default:
    return fetch(event.request);
  }
}

*/