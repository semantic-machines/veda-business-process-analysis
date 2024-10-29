// Register service worker and install PWA

if ('serviceWorker' in navigator) {
  const HTML = `
    <div class="container text-center" id="install-app" style="display:none; margin:0.75rem auto;">
      Установить приложение на главный экран? Install the application on the main screen?
      <button id="install-btn" class="btn btn-sm btn-primary margin-md margin-md-h">Установить / Install</button>
      <button id="reject-install-btn" class="btn btn-sm btn-link" style="margin-left:0;padding-left:0;">Отказаться / Refuse</button>
    </div>
  `;
  const wrapper = document.createElement('div');
  wrapper.id = 'install-wrapper';
  wrapper.innerHTML = HTML;
  document.body.insertBefore(wrapper, document.body.firstChild);

  // Install SW
  navigator.serviceWorker.register('ServiceWorker.js', {scope: window.location})
    .then((registration) => {
      console.log('Service worker registered:', registration.scope);
    })
    .catch((error) => console.error('Service worker registration failed'));

  // Install application prompt
  const showAddToHomeScreen = () => {
    const installApp = document.getElementById('install-app');
    const installBtn = document.getElementById('install-btn');
    const rejectInstallBtn = document.getElementById('reject-install-btn');
    installApp.style.display = 'block';
    installBtn.addEventListener('click', addToHomeScreen);
    rejectInstallBtn.addEventListener('click', rejectInstall);
  };

  const addToHomeScreen = () => {
    const installApp = document.getElementById('install-app');
    installApp.style.display = 'none'; // Hide the prompt
    deferredPrompt.prompt(); // Wait for the user to respond to the prompt
    deferredPrompt.userChoice
      .then((choiceResult) => {
        if (choiceResult.outcome === 'accepted') {
          console.log('User accepted install prompt');
        } else {
          console.log('User dismissed install prompt');
        }
        deferredPrompt = null;
      });
  };

  const rejectInstall = () => {
    const installApp = document.getElementById('install-app');
    installApp.style.display = 'none';
    localStorage.rejectedInstall = true;
  };

  let deferredPrompt;
  window.addEventListener('beforeinstallprompt', (e) => {
    // Prevent Chrome 67 and earlier from automatically showing the prompt
    e.preventDefault();
    // Stash the event so it can be triggered later.
    deferredPrompt = e;
    if (!localStorage.rejectedInstall) {
      showAddToHomeScreen();
    }
  });
}
