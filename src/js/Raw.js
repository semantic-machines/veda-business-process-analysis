import {Component, html, safe} from 'veda-client';

export default class Raw extends Component(HTMLElement) {
  static tag = 'bpa-raw';

  format = localStorage.getItem('Raw_format') === 'ttl' ? 'ttl' : 'json';
  editing = false;

  toggleFormat() {
    this.format = this.format === 'ttl' ? 'json' : 'ttl';
    localStorage.setItem('Raw_format', this.format);
    this.update();
  }

  async toggleEdit() {
    this.editing = !this.editing;
    await this.update();
    if (this.editing) {
      const textarea = this.querySelector('textarea');
      textarea.focus();
    }
  }

  async saveChanges() {
    try {
      const textarea = this.querySelector('textarea');
      const json = JSON.parse(textarea.value);
      await this.model.apply(json);
      await this.model.save();
      this.editing = false;
      this.update();
    } catch (error) {
      console.error('Ошибка при сохранении:', error);
      alert('Ошибка при сохранении изменений. Проверьте формат JSON.');
    }
  }

  async remove() {
    if (confirm('Вы действительно хотите удалить этот объект?')) {
      try {
        await this.model.remove();
        history.back();
      } catch (error) {
        console.error('Ошибка при удалении объекта:', error);
        alert('Ошибка при удалении объекта. Попробуйте позже.');
      }
    }
  }

  render() {
    return html`
      <style>
        .fullscreen-editor {
          position: fixed;
          top: 0.25em;
          left: 0.25em;
          right: 0.25em;
          bottom: 0.25em;
          z-index: 1000;
        }
        .fullscreen-editor textarea {
          font-size: 0.85em;
          width: 100%;
          height: calc(100%);
          border: none;
          padding: 1em 1em 2em 1em;
        }
        .bottom-controls {
          position: fixed;
          bottom: 0;
          left: 0;
          right: 0;
          z-index: 1001;
          padding: 1em;
          text-align: center;
        }
        .bottom-controls button {
          margin: 0 0.5em;
        }
      </style>
      ${this.editing ? html`
        <div class="fullscreen-editor">
          <textarea class="font-monospace form-control"></textarea>
          <div class="bottom-controls">
            <button type="button"
              class="btn btn-success"
              on:click="${(e) => this.saveChanges(e)}">
              Сохранить
            </button>
            <button type="button"
              class="btn btn-secondary"
              on:click="${(e) => this.toggleEdit(e)}">
              Отмена
            </button>
          </div>
        </div>
      ` : html`
        <div class="mb-2 ms-3 d-flex justify-content-between">
          <div class="nav nav-underline" role="group">
            <button type="button"
              class="nav-link ${this.format === 'ttl' ? 'active disabled' : 'text-secondary'}"
              on:click="${(e) => this.toggleFormat(e)}">
              TTL
            </button>
            <button type="button"
              class="nav-link ${this.format === 'json' ? 'active disabled' : 'text-secondary'}"
              on:click="${(e) => this.toggleFormat(e)}">
              JSON
            </button>
          </div>
          <div>
            <button type="button" class="btn text-secondary" on:click="${(e) => this.remove(e)}">
              <span about="v-bpa:Remove" property="rdfs:label"></span>
            </button>
            ${this.format === 'json' ? html`
              <button type="button" class="btn text-dark" on:click="${(e) => this.toggleEdit(e)}">
                <span about="v-bpa:Edit" property="rdfs:label"></span>
              </button>
            ` : ''}
          </div>
        </div>
        <div class="sheet">
          <pre class="m-0"><code></code></pre>
        </div>
      `}
    `;
  }

  post() {
    if (this.editing) {
      this.querySelector('textarea').innerHTML = toJSON(this.model);
    } else {
      this.querySelector('code').innerHTML = this.format === 'ttl' ? toTurtle(this.model) : toJSON(this.model);
    }
  }

  added() {
    this.model.on('modified', this.modifiedHandler);
  }

  modifiedHandler = () => {
    this.update();
  }

  removed() {
    this.model.off('modified', this.modifiedHandler);
  }
}

customElements.define(Raw.tag, Raw);

function toJSON(model) {
  return safe(JSON.stringify(model, null, 2))
}

function toTurtle(model) {
  return Object.entries(model).map(([predicate, objects]) => {
    if (predicate === 'id') return `<b>${safe(objects)}</b>`;
    return objects.map(obj => {
      if (typeof obj === 'object' && obj.id) {
        return `  <a class="link-secondary" href="#/Raw/${safe(predicate)}">${safe(predicate)}</a> <a class="link" href="#/Raw/${safe(obj.id)}">${safe(obj.id)}</a> ;`;
        } else if (typeof obj === 'string') {
          return `  <a class="link-secondary" href="#/Raw/${safe(predicate)}">${safe(predicate)}</a> "${safe(obj)}" ;`;
        } else {
          return `  <a class="link-secondary" href="#/Raw/${safe(predicate)}">${safe(predicate)}</a> ${safe(obj)} ;`;
        }
      }).join('\n');
    }).filter(Boolean).join('\n');
}

const setOutline = (e) => {
  if ((e.altKey && e.ctrlKey) || (e.metaKey && e.altKey)) {
    e.preventDefault();
    e.stopPropagation();

    const about = e.target.closest('[about]');
    if (about) {
      about.style.outline = '2px solid #007bff';
      about.title = about.getAttribute('about');
      about.classList.add('outlined');
    }
  }
};

const removeOutline = (e) => {
  if ((e.altKey && e.ctrlKey) || (e.metaKey && e.altKey)) {
    e.preventDefault();
    e.stopPropagation();

    const about = e.target.closest('.outlined[about]');
    if (about) {
      about.style.outline = '';
      about.title = '';
      about.classList.remove('outlined');
    }
  }
};

const handleKeyUp = (e) => {
  if (['Alt','Control','Meta'].includes(e.key)) {
    Array.from(document.querySelectorAll('.outlined[about]')).forEach(about => {
      about.style.outline = '';
      about.title = '';
      about.classList.remove('outlined');
    });
  }
};

const handleClick = (e) => {
  if ((e.altKey && e.ctrlKey) || (e.metaKey && e.altKey)) {
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();

    const about = e.target.closest('[about]');
    if (about) {
      location.hash = `#/Raw/${about.getAttribute('about')}`;
    }
  }
};

document.addEventListener('mouseover', setOutline, true);
document.addEventListener('mouseout', removeOutline, true);
document.addEventListener('keyup', handleKeyUp, true);
document.addEventListener('click', handleClick, true);
