import {Component, html, Backend, Model} from 'veda-client';
import BusinessProcessList from './BusinessProcessList.js';
import ProcessClusterList from './ProcessClusterList.js';

export default class BusinessProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  added() {
    this.showClusters = false;
    this.transitioning = false;
  }

  async toggleView() {
    if (this.transitioning) return;
    this.transitioning = true;

    const container = this.querySelector('.view-container');
    container.style.opacity = '0';

    await new Promise(resolve => setTimeout(resolve, 300));
    
    this.showClusters = !this.showClusters;
    this.update();

    requestAnimationFrame(() => {
      container.style.opacity = '1';
    });

    setTimeout(() => {
      this.transitioning = false;
    }, 300);
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

      <div class="view-container" style="transition: opacity 0.3s">
        ${this.showClusters ? 
          html`<${ProcessClusterList}></${ProcessClusterList}>` :
          html`<${BusinessProcessList}></${BusinessProcessList}>`
        }
      </div>
    `;
  }
}

customElements.define(BusinessProcessOverview.tag, BusinessProcessOverview);
