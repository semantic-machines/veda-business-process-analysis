const VERSION = 1732732800602;
const FILES = `files-${VERSION}`;
const API = `api-${VERSION}`;

const FILES_TO_CACHE = [
  'bootstrap-icons-OCU552PF.woff',
  'bootstrap-icons-X6UQXWUS.woff2',
  'calibri-6HZ4FOZK.otf',
  'calibri-CUMK4OKU.woff2',
  'calibri-B45TIHX4.woff',
  'calibri-WWLRWMHF.ttf',
  'favicon.ico',
  'index.css',
  'index.html',
  'index.js',
  'index.js.map',
  'manifest',
];

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

// Clear cached resources
self.addEventListener('install', (event) => {
  self.skipWaiting();
  console.log('Service worker installed, caching files');
  event.waitUntil(
    caches.open(FILES).then((cache) => {
      return Promise.all(
        FILES_TO_CACHE.map((file) => {
          return cache.add(file).catch((error) => {
            console.error(`Failed to cache ${file}:`, error);
            throw error;
          });
        })
      );
    })
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
  console.log('Service worker activated, cleaning up old caches');
  const cacheWhitelist = [FILES, API];
  event.waitUntil(
    caches.keys().then((keyList) => {
      return Promise.all(
        keyList.map((key) => {
          if (!cacheWhitelist.includes(key)) {
            console.log(`Deleting cache: ${key}`);
            return caches.delete(key);
          }
        })
      );
    })
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
