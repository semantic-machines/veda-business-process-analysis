import {Component, html, Router, Model} from 'veda-client';

const router = new Router;

export default class Breadcrumb extends Component(HTMLElement) {
  static toString () {
    return 'bpa-breadcrumb';
  }
  static get observedAttributes () {
    return ['about'];
  }
  attributeChangedCallback (name, prev, curr) {
    if (curr !== prev && prev) {
      this.model = new Model(curr);
      this.update();
    }
  }
  added () {
    location.hash.replace(/#\/([^/]+)/g, (_, id) => this.model = new Model(id));
    router.add('#/:id', (id) => this.setAttribute('about', id));
  }
  async render () {
    return html`
      <div class="container my-2">
        <nav aria-label="breadcrumb" style="--bs-breadcrumb-divider: '>';">
          <strong>
            <ol class="breadcrumb mb-0" is="${BreadcrumbItem}" about="${this.model.id}"></ol>
          </strong>
        </nav>
      </div>
    `;
  }
}
customElements.define(Breadcrumb.toString(), Breadcrumb);

class BreadcrumbItem extends Component(HTMLOListElement) {
  static toString () {
    return 'bpa-breadcrumb-item';
  }
  async render () {
    let model = this.model;
    const items = [];
    while (true) {
      items.push(html`
        <li class="breadcrumb-item d-none d-sm-block">
          <a href="#/${model.id}" about="${model.id}" property="rdfs:label" style="color:black;"></a>
        </li>
      `);
      if (model.hasValue('portal:hasParentSection')) {
        model = model['portal:hasParentSection'][0];
        await model.load();
      } else break;
    }
    return items.reverse().join('');
  }
}
customElements.define(BreadcrumbItem.toString(), BreadcrumbItem, {extends: 'ol'});
