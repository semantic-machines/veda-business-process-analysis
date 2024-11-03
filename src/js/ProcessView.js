import {Component, html} from 'veda-client';

export default class ProcessView extends Component(HTMLElement) {
  static tag = 'bpa-process-view';
  
  render() {
    return html`
      <small><pre><code></code></pre></small>
    `;
  }
  post() {
    const turtle = Object.entries(this.model).map(([predicate, objects]) => {
      if (!Array.isArray(objects)) return '';
      return objects.map(obj => {
        if (typeof obj === 'object' && obj.id) {
          return `  ${predicate} ${obj.id} ;`;
        } else if (typeof obj === 'string') {
          return `  ${predicate} "${obj}" ;`; 
        } else {
          return `  ${predicate} ${obj} ;`;
        }
      }).join('\n');
    }).filter(Boolean).join('\n');
    
    this.querySelector('code').innerHTML = `<${this.model.id}>\n${turtle}\n.`;
  }
}

customElements.define(ProcessView.tag, ProcessView);
