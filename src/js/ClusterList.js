import {Component, html} from 'veda-client';
import Literal from './Literal.js';
import ProcessJustificationIndicator from './ProcessJustificationIndicator';
import ClusterizationButton from './ClusterizationButton.js';
import PlaceholderRow from './PlaceholderRow.js';

export default class ClusterList extends Component(HTMLElement) {
  static tag = 'bpa-cluster-list';

  added() {
    this.running = this.getAttribute('running');
    this.callback = this.getAttribute('callback');
  }

  async post () {
    this.addEventListener('click', (e) => {
      const toggleBtn = e.target.closest('.toggle-processes');
      if (!toggleBtn) return;

      e.stopPropagation();
      e.preventDefault();

      [...toggleBtn.children].forEach(el => el.classList.toggle('d-none'));
      const id = toggleBtn.dataset.for;
      const processes = toggleBtn.parentNode.parentNode.querySelector(`[data-id="${id}"]`);
      processes.classList.toggle('d-none');

      const isExpanded = !processes.classList.contains('d-none');
      localStorage.setItem(`ClusterList_expanded_${id}`, isExpanded);
    }, true);

    this.addEventListener('click', (e) => {
      const processRow = e.target.closest('.process-row');
      const about = processRow?.getAttribute('about');
      if (about) location.hash = `#/ProcessView/${about}`;
    });

    this.addEventListener('click', (e) => {
      const clusterRow = e.target.closest('.cluster-row');
      const about = clusterRow?.getAttribute('about');
      if (about) location.hash = `#/ClusterView/${about}`;
    });
  }

  render() {
    let isExpanded;
    return html`
      <style>
        ${ClusterList} #processes-table tbody tr:last-child {
          border-bottom: 1px solid transparent;
        }
        ${ClusterList} tr:not([disabled]) {
          cursor: pointer;
        }
        ${ClusterList} tr.expanded-processes:hover > td {
          box-shadow: none;
        }
      </style>
      <div class="sheet">
        <div class="d-flex align-items-center justify-content-between">
          <div class="d-flex align-items-center">
            <i class="bi bi-collection me-3 fs-1"></i>
            <h3 about="v-bpa:ProcessClusters" property="rdfs:label" class="mb-1"></h3>
          </div>
          <${ClusterizationButton} ${this.running ? html`about="${this.running}"` : ''} callback="${this.callback}" class="ms-auto"></${ClusterizationButton}>
        </div>
        <div class="table-responsive">
          <table class="table table-hover mb-0 table-borderless" id="clusters-table">
            <thead>
              <tr>
                <th width="0%" class="text-secondary fw-normal"></th>
                <th width="40%" class="text-secondary fw-normal" about="v-bpa:ProcessCluster" property="rdfs:label"></th>
                <th width="30%" class="text-secondary fw-normal" about="v-bpa:clusterSimilarities" property="rdfs:label"></th>
                <th width="30%" class="text-secondary fw-normal" about="v-bpa:clusterDifferences" property="rdfs:label"></th>
              </tr>
            </thead>
            <tbody>
              ${this.model?.['v-bpa:foundClusters']?.map((cluster) => html`
                ${(isExpanded = localStorage.getItem(`ClusterList_expanded_${cluster.id}`) === 'true', '')}
                <tr about="${cluster.id}" class="border-top cluster-row" is="${PlaceholderRow}" when="!this.model.hasValue('rdfs:label')" rows="3">
                  <td class="text-center toggle-processes" data-for="${cluster.id}">
                    <i class="bi bi-chevron-up text-secondary ${isExpanded ? '' : 'd-none'}"></i>
                    <span class="${isExpanded ? 'd-none' : ''}">
                      <span class="badge bg-success-subtle text-dark">{{this.model['v-bpa:hasProcess']?.length}}</span>
                      <i class="bi bi-chevron-down text-secondary"></i>
                    </span>
                  </td>
                  <td class="align-middle">
                    <p class="mb-0 fw-bold" property="rdfs:label"></p>
                    <p class="mb-0 text-secondary">
                      <${Literal} about="${cluster.id}" property="v-bpa:proposedClusterDescription" max-chars="70"></${Literal}>
                    </p>
                  </td>
                  <td class="align-middle">
                    <i class="bi bi-intersect fs-6 me-2 text-secondary"></i>
                    <${Literal} about="${cluster.id}" property="v-bpa:clusterSimilarities" max-chars="70"></${Literal}>
                  </td>
                  <td class="align-middle">
                    <i class="bi bi-exclude fs-6 me-2 text-secondary"></i>
                    <${Literal} about="${cluster.id}" property="v-bpa:clusterDifferences" max-chars="70"></${Literal}>
                  </td>
                </tr>
                <tr about="${cluster.id}" class="expanded-processes ${isExpanded ? '' : 'd-none'}" data-id="${cluster.id}">
                  <td></td>
                  <td colspan="3" class="p-0">
                    <div class="table-responsive">
                      <table class="table mb-0 table-hover table-light table-borderless" id="processes-table">
                        <thead>
                          <tr class="border-bottom">
                            <th width="40%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                            <th width="20%" class="text-secondary fw-normal" about="v-bpa:hasProcessJustification" property="rdfs:label"></th>
                            <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                            <th width="10%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                            <th width="10%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
                          </tr>
                        </thead>
                        <tbody rel="v-bpa:hasProcess">
                          <tr about="{{this.model.id}}" class="process-row">
                            <td width="40%" class="align-middle">
                              <h5 class="mb-0" property="rdfs:label"></h5>
                              <p class="text-muted mb-0">
                                <${Literal} about="{{this.model.id}}" property="v-bpa:processDescription" max-chars="70"></${Literal}>
                              </p>
                            </td>
                            <td width="20%" class="align-middle" rel="v-bpa:hasProcessJustification">
                              <${ProcessJustificationIndicator} class="text-nowrap" about="{{this.model.id}}" property="rdfs:label"></${ProcessJustificationIndicator}>
                            </td>
                            <td width="20%" class="align-middle" property="v-bpa:responsibleDepartment"></td>
                            <td width="10%" class="align-middle">
                              <i class="bi bi-people-fill me-1"></i>
                              <strong>{{ this.model.hasValue('v-bpa:processParticipant') && typeof this.model['v-bpa:processParticipant'][0] === 'string' ? this.model['v-bpa:processParticipant'][0].split(',').length : 0 }}</strong>
                            </td>
                            <td width="10%" class="align-middle lh-sm">
                              <strong>{{ this.model.hasValue('v-bpa:laborCosts') && this.model.hasValue('v-bpa:processFrequency') ? this.model['v-bpa:laborCosts'][0] * this.model['v-bpa:processFrequency'][0] : 0 }}</strong>
                              <br>
                              <small class="text-secondary" about="v-bpa:HoursPerYear" property="rdfs:comment"></small>
                            </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </td>
                </tr>
              `).join('') ?? ''}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }
}

customElements.define(ClusterList.tag, ClusterList);
