import {Component, html} from 'veda-client';

export default class TTLView extends Component(HTMLElement) {
  static tag = 'bpa-ttl-view';

  format = sessionStorage.getItem('TTLView_format') === 'ttl' ? 'ttl' : 'json';

  toggleFormat() {
    this.format = this.format === 'ttl' ? 'json' : 'ttl';
    sessionStorage.setItem('TTLView_format', this.format);
    this.update();
  }


  render() {
    return html`
      <div class="nav nav-underline mb-2 ms-3" role="group">
        <button type="button" 
          class="nav-link ${this.format === 'ttl' ? 'active disabled' : 'text-secondary'}" 
          @click="toggleFormat">
          TTL
        </button>
        <button type="button"
          class="nav-link ${this.format === 'json' ? 'active disabled' : 'text-secondary'}"
          @click="toggleFormat">
          JSON
        </button>
      </div>
      <div class="sheet">
        <pre class="m-0"><code></code></pre>
      </div>
    `;
  }
  
  post() {
    this.querySelector('code').innerHTML = this.format === 'ttl' ? 
      `${toTurtle(this.model)}` :
      `${toJSON(this.model)}`;
  }
}

customElements.define(TTLView.tag, TTLView);

document.addEventListener('mousemove', (event) => {
  if (event.altKey && event.ctrlKey || event.metaKey && event.altKey) {
    event.preventDefault();
    event.stopPropagation();
    
    // Убираем предыдущие подсветки и title
    document.querySelectorAll('[about]').forEach(el => {
      el.style.outline = 'none';
      el.removeAttribute('title');
    });

    // Находим элемент с атрибутом about под курсором
    const elementUnderCursor = document.elementFromPoint(event.clientX, event.clientY);
    const targetElement = elementUnderCursor?.closest('[about]');
    
    if (targetElement) {
      targetElement.style.outline = '2px solid #007bff';
      targetElement.style.outlineOffset = '2px';
      targetElement.title = targetElement.getAttribute('about');
    }
  }
}, {capture: true});

document.addEventListener('click', (event) => {
  if (event.altKey && event.ctrlKey || event.metaKey && event.altKey) {
    event.preventDefault();
    event.stopPropagation();
    const elementUnderCursor = document.elementFromPoint(event.clientX, event.clientY);
    const targetElement = elementUnderCursor?.closest('[about]');
    
    if (targetElement) {
      const about = targetElement.getAttribute('about');
      window.location.hash = `#/TTLView/${about}`;
    }
  }
}, {capture: true});

document.addEventListener('keyup', (event) => {
  if (event.key === 'Alt' || event.key === 'Control' || event.key === 'Meta') {
    // Убираем все подсветки и title при отпускании клавиш
    document.querySelectorAll('[about]').forEach(el => {
      el.style.outline = 'none';
      el.removeAttribute('title');
    });
  }
});

function toJSON(model) {
  return JSON.stringify(model, null, 2)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function toTurtle(model) {
  return Object.entries(model).map(([predicate, objects]) => {
    if (predicate === 'id') return `<b>${objects}</b>`;
    return objects.map(obj => {
      if (typeof obj === 'object' && obj.id) {
        return `  <a class="link-secondary" href="#/TTLView/${predicate}">${predicate}</a> <a class="link" href="#/TTLView/${obj.id}">${obj.id}</a> ;`;
        } else if (typeof obj === 'string') {
          return `  <a class="link-secondary" href="#/TTLView/${predicate}">${predicate}</a> "${obj}" ;`; 
        } else {
          return `  <a class="link-secondary" href="#/TTLView/${predicate}">${predicate}</a> ${obj} ;`;
        }
      }).join('\n');
    }).filter(Boolean).join('\n');
}