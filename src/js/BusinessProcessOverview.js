import {Component, html, timeout} from 'veda-client';
import BusinessProcessList from './BusinessProcessList.js';
import ClusterList from './ClusterList.js';

export default class BusinessProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  showClusters = sessionStorage.getItem('showClusters') === 'true';

  toggleView() {
    this.showClusters = !this.showClusters;
    sessionStorage.setItem('showClusters', this.showClusters);
    this.update();
  }

  render() {
    return html`
      <div class="mb-2 ms-3 d-flex justify-content-between">
        <ul class="nav nav-underline">
          <li class="nav-item">
            <button @click="toggleView" class="nav-link ${!this.showClusters ? 'active disabled' : 'text-secondary'}">
              <span about="v-bpa:ShowProcesses" property="rdfs:label"></span>
            </button>
          </li>
          <li class="nav-item">
            <button @click="toggleView" class="nav-link ${this.showClusters ? 'active disabled' : 'text-secondary'}">
              <span about="v-bpa:ShowClusters" property="rdfs:label"></span>
            </button>
          </li>
        </ul>
        ${this.showClusters 
          ? html`<button class="btn text-dark"><i class="bi bi-arrow-repeat"></i> Обновить кластеры</button>`
          : html`<button class="btn text-dark"><i class="bi bi-plus"></i> Добавить</button>`
        }
      </div>
      ${this.showClusters ? 
        html`<${ClusterList}></${ClusterList}>` :
        html`<${BusinessProcessList}></${BusinessProcessList}>`
      }
    `;
  }
}

customElements.define(BusinessProcessOverview.tag, BusinessProcessOverview);
