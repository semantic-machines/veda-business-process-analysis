import {Component, html} from 'veda-client';
import {toTurtle} from './Util';

export default class ClusterView extends Component(HTMLElement) {
  static tag = 'bpa-cluster-view';
  
  render() {
    return html`
      <div class="sheet">
        <pre class="mb-0"><code></code></pre>
      </div>
    `;
  }
  
  post() {
    this.querySelector('code').innerHTML = `${toTurtle(this.model)}`;
  }
}

customElements.define(ClusterView.tag, ClusterView);
