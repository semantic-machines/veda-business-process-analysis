import {Component, html, Backend, Model} from 'veda-client';

export default class ClusterizationButton extends Component(HTMLElement) {
  static tag = 'bpa-clusterization-button';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:CompletedAndRunningClusterizationAttempts';
    params['v-s:resultFormat'] = 'cols';
    try {
      const {running: [attempt]} = await Backend.stored_query(params);
      if (attempt) this.attempt = new Model(attempt);
    } catch (e) {
      console.error('Error querying running clusterization attempts', e);
    }
  }

  async updateClusters() {
    const attempt = new Model();
    attempt['rdf:type'] = ['v-bpa:ClusterizationAttempt'];
    attempt['v-bpa:controlAction'] = ['v-bpa:StartExecution'];
    try {
      await attempt.save();
    } catch (e) {
      console.error('Error saving clusterization attempt', e);
      alert('Ошибка при запуске кластеризации');
    }
    this.attempt = attempt;
    this.update();
  }

  render() {
    return html`
      <button class="btn btn-light" @click="updateClusters" ${this.attempt ? 'disabled' : ''}>
        ${this.attempt 
          ? `<${Attempt} about=${this.attempt.id}></${Attempt}>`
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
    const progress = this.model?.['v-bpa:clusterizationProgress']?.[0];

    return html`
      ${state === 'v-bpa:ExecutionCompleted' 
        ? '<i class="bi bi-check-circle-fill text-success me-2"></i>'
        : `<div class="spinner-grow spinner-grow-sm me-2" role="status">
            <span class="visually-hidden">Loading...</span>
          </div>`
      }
      ${state ? `<span about="${state}" property="rdfs:label" class="me-1"></span>` : ''}
      ${progress ? `${progress}%` : ''}
    `
  }
}

customElements.define(Attempt.tag, Attempt);
