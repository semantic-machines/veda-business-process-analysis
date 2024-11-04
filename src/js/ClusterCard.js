import {Component, html} from 'veda-client';
import ProcessCard from './ProcessCard.js';

export default class ClusterCard extends Component(HTMLElement) {
  static tag = 'bpa-cluster-card';

  getStorageKey() {
    return `cluster-expanded-${this.getAttribute('about')}`;
  }

  expanded = sessionStorage.getItem(this.getStorageKey()) === 'true';

  toggleExpanded(e) {
    e.preventDefault();
    this.expanded = !this.expanded;
    sessionStorage.setItem(this.getStorageKey(), this.expanded);
    this.update();
  }

  async render() {
    return html`
      <style>
        a:hover > .cluster-card {
          background-color: #f5f5f5;
        }
        a:hover > .cluster-card .business-process-card {
          background-color: #e9e9e9;
        }
        a > .cluster-card .business-process-card:hover {
          background-color: #e0e0e0;
        }
        a > .cluster-card {
          background-color: #ffffff;
          border: none;
        }
        .toggle-expand:hover {
          background-color: #f5f5f5;
        }
      </style>
      <div class="d-flex align-items-stretch">
        <div class="toggle-expand d-flex flex-column justify-content-center align-items-center px-2" style="cursor: pointer;" @click="toggleExpanded">
          <span class="badge bg-success-subtle text-dark mb-1">
            ${this.model['v-bpa:hasProcess']?.length ?? 0}
          </span>
          <i class="bi bi-chevron-${this.expanded ? 'up' : 'down'}"></i>
        </div>
        <a href="#/ClusterView/${this.model.id}" style="text-decoration: none;" class="flex-grow-1">
          <div class="cluster-card card">
            <div class="card-body position-relative">
              <div class="d-flex align-items-center justify-content-between mb-2">
                <div>
                <h4 property="rdfs:label" class="mb-0"></h4>
                <div class="text-muted">
                  <small property="v-bpa:clusterSimilarities"></small>
                </div>
              </div>
              <div class="text-end">
                <strong>${this.dataset.totalTime}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span></strong>
              </div>
            </div>
            <div class="mt-2 d-flex justify-content-between align-items-center">
              <div>
                <span class="badge text-bg-secondary border border-secondary me-2" property="v-bpa:proposedDepartment"></span>
                <span class="badge text-bg-light border border-secondary text-muted">
                  <i class="bi bi-arrow-repeat me-1"></i>
                  <span property="v-bpa:proposedFrequency"></span>
                  &nbsp;<span about="v-bpa:TimesPerYear" property="rdfs:label"></span>
                </span>
              </div>
              <div>
                <small property="v-bpa:proposedParticipants"></small>
              </div>
            </div>
            ${this.expanded 
              ? html`
                <div rel="v-bpa:hasProcess" class="ms-4 mt-3 d-flex flex-column gap-3">
                  <${ProcessCard} about={{this.model.id}}></${ProcessCard}>
                </div>`
              : ''}
          </div>
        </div>
      </a>
    `;
  }
}

customElements.define(ClusterCard.tag, ClusterCard);
