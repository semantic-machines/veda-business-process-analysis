import {Component, html, safe, Model} from 'veda-client';
import Literal from './Literal';
import Callback from './Callback.js';

export default class ClusterizationButton extends Component(HTMLElement) {
  static tag = 'bpa-clusterization-button';

  added() {
    this.callback = Callback.get(this.getAttribute('callback'));
  }

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
      ${this.model
        ? html`<${Attempt} about="${this.model.id}"></${Attempt}>`
        : html`
          <button class="btn btn-link text-dark text-decoration-none" on:click="${(e) => this.updateClusters(e)}" ${this.model ? 'disabled' : ''}>
            <i class="bi bi-arrow-repeat me-2"></i><span about="v-bpa:UpdateClusters" property="rdfs:label"></span>
          </button>`
      }
    `;
  }
}

customElements.define(ClusterizationButton.tag, ClusterizationButton);

class Attempt extends Component(HTMLElement) {
  static tag = 'bpa-attempt';

  added() {
    this.model.on('modified', this.handler);
  }

  handler = () => this.update();

  removed() {
    this.model.off('modified', this.handler);
  }

  render() {
    const state = this.model?.['v-bpa:hasExecutionState']?.[0].id;
    const progress = this.model?.['v-bpa:clusterizationProgress']?.[0] ?? 0;

    return html`
      <div class="d-flex align-items-center">
        ${state === 'v-bpa:ExecutionCompleted'
          ? html`
            <i class="bi bi-check-circle-fill text-success me-2"></i>
            <${Literal} about="${state}" property="rdfs:label" class="me-1"></${Literal}>`
          : state === 'v-bpa:ExecutionError'
          ? html`
            <i class="bi bi-exclamation-circle-fill text-danger me-2"></i>
            <${Literal} about="${state}" property="rdfs:label" class="me-1" title="${this.model['v-bpa:lastError'][0]}"></${Literal}>`
          : html`
            <div class="spinner-grow spinner-grow-sm me-2" role="status">
              <span class="visually-hidden">Loading...</span>
            </div>
            ${state ? html`<${Literal} class="me-2" about="${state}" property="rdfs:label"></${Literal}>` : ''}
            <div class="progress d-inline-block border border-tertiary-subtle" role="progressbar" aria-label="Clusterization progress" aria-valuenow="${progress}" aria-valuemin="0" aria-valuemax="100" style="height: 16px; width: 60px">
              <div class="progress-bar fw-bold progress-bar-striped progress-bar-animated bg-success" style="width: ${progress}%">${progress}%</div>
            </div>`
        }
      </div>
    `;
  }
}

customElements.define(Attempt.tag, Attempt);
