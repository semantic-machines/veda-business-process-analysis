import {Component, html, Model} from 'veda-client';
import Literal from './Literal';
import Callback from './Callback.js';

export default class ClusterizationButton extends Component(HTMLElement) {
  static tag = 'bpa-clusterization-button';

  callback = Callback.get(this.getAttribute('callback'));

  async updateClusters() {
    const model = new Model();
    model['rdf:type'] = ['v-bpa:ClusterizationAttempt'];
    model['v-bpa:controlAction'] = ['v-bpa:StartExecution'];
    try {
      await model.save();
      this.callback?.(model);
    } catch (e) {
      console.error('Error saving clusterization attempt', e);
      alert('Ошибка при запуске кластеризации');
    }
    this.model = model;
    this.update();
  }

  render() {
    return html`
      <button class="btn btn-link text-dark text-decoration-none" @click="${(e) => this.updateClusters(e)}" ${this.model ? 'disabled' : ''}>
        ${this.model 
          ? `<${Attempt} about=${this.model.id}></${Attempt}>`
          : `<i class="bi bi-arrow-repeat me-2"></i><span about="v-bpa:UpdateClusters" property="rdfs:label"></span>`
        }
      </button>
    `;
  }
}

customElements.define(ClusterizationButton.tag, ClusterizationButton);

class Attempt extends Component(HTMLElement) {
  static tag = 'bpa-attempt';

  handler = () => this.update();

  added() {
    this.model.on('modified', this.handler);
  }
  
  removed() {
    this.model.off('modified', this.handler);
  }

  render() {
    const state = this.model?.['v-bpa:hasExecutionState']?.[0].id;
    const progress = this.model?.['v-bpa:clusterizationProgress']?.[0] ?? 0;
    const secondsLeft = this.model?.['v-bpa:estimatedTime']?.[0];

    let timeString = '';
    if (secondsLeft) {
      timeString = ', осталось<i class="bi bi-clock-history mx-1"></i>';
      const minutes = Math.floor(secondsLeft / 60);
      const seconds = secondsLeft % 60;
      timeString += `${minutes}:${seconds.toString().padStart(2, '0')}`;
    }

    return html`
      ${state === 'v-bpa:ExecutionCompleted' 
        ? `<i class="bi bi-check-circle-fill text-success me-2"></i>
           <${Literal} about="${state}" property="rdfs:label" class="me-1"></${Literal}>`
        : `
          <div class="spinner-grow spinner-grow-sm me-2" role="status">
            <span class="visually-hidden">Loading...</span>
          </div>
          ${state ? `<${Literal} about="${state}" property="rdfs:label"></${Literal}>:` : ''}
          <span class="ms-1">${progress}%</span>${timeString}
          `
      }
    `;
  }
}

customElements.define(Attempt.tag, Attempt);

