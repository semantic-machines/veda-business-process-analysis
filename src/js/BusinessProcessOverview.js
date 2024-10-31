import {Component, html, timeout} from 'veda-client';
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

    const container = this.lastElementChild;
    container.style.opacity = '0';

    await timeout(200);
    
    this.showClusters = !this.showClusters;
    this.update();

    requestAnimationFrame(() => {
      container.style.opacity = '1';
    });

    setTimeout(() => {
      this.transitioning = false;
    }, 200);
  }

  render() {
    return html`
      <div class="mb-3">
        <div class="form-check form-switch">
          <input class="form-check-input" type="checkbox" role="switch" 
            id="viewSwitch" @change="toggleView" ${this.showClusters ? 'checked' : ''}>
          <label class="form-check-label" for="viewSwitch" about="v-bpa:ShowClusters" property="rdfs:label"></label>
        </div>
      </div>

      <div style="transition: opacity 0.3s">
        ${this.showClusters ? 
          html`<${ProcessClusterList}></${ProcessClusterList}>` :
          html`<${BusinessProcessList}></${BusinessProcessList}>`
        }
      </div>
    `;
  }
}

customElements.define(BusinessProcessOverview.tag, BusinessProcessOverview);
