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

const handleMouseMove = (e) => {
  if ((e.altKey && e.ctrlKey) || (e.metaKey && e.altKey)) {
    e.preventDefault();
    e.stopPropagation();
    
    document.querySelectorAll('[about]').forEach(el => {
      el.style.outline = '';
      el.removeAttribute('title'); 
    });

    const target = e.target.closest('[about]');
    if (target) {
      target.style.outline = '2px solid #007bff';
      target.style.outlineOffset = '2px';
      target.title = target.getAttribute('about');
    }
  }
};

const handleClick = (e) => {
  if ((e.altKey && e.ctrlKey) || (e.metaKey && e.altKey)) {
    e.preventDefault(); 
    const target = e.target.closest('[about]');
    if (target) {
      location.hash = `#/TTLView/${target.getAttribute('about')}`;
    }
  }
};

const handleKeyUp = (e) => {
  if (['Alt','Control','Meta'].includes(e.key)) {
    document.querySelectorAll('[about]').forEach(el => {
      el.style.outline = '';
      el.removeAttribute('title');
    });
  }
};

document.addEventListener('mousemove', handleMouseMove, true);
document.addEventListener('click', handleClick, true);
document.addEventListener('keyup', handleKeyUp);

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