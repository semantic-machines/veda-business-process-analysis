import {Component, html, Backend, Model} from 'veda-client';
import BusinessProcessList from './BusinessProcessList.js';
import ProcessClusterList from './ProcessClusterList.js';

export default class BusinessProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  added() {
    this.showClusters = false;
  }

  toggleView() {
    this.showClusters = !this.showClusters;
    this.update();
  }

  render() {
    return html`
      <div class="mb-3">
        <div class="form-check form-switch">
          <input class="form-check-input" type="checkbox" role="switch" 
            id="viewSwitch" @change="toggleView" ${this.showClusters ? 'checked' : ''}>
          <label class="form-check-label" for="viewSwitch">Показать кластеры</label>
        </div>
      </div>

      ${this.showClusters ? 
        html`<${ProcessClusterList}></${ProcessClusterList}>` :
        html`<${BusinessProcessList}></${BusinessProcessList}>`
      }
    `;
  }
}

customElements.define(BusinessProcessOverview.tag, BusinessProcessOverview);
