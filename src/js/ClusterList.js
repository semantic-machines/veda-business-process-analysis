import {Component, html, Backend, Model, timeout} from 'veda-client';
import Literal from './Literal.js';
import ProcessJustificationIndicator from './ProcessJustificationIndicator';


export default class ClusterList extends Component(HTMLElement) {
  static tag = 'bpa-cluster-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessClusters';
    const {rows: clusters} = await Backend.stored_query(params);
    this.clusters = clusters;
  }
  
  async post () {
    await timeout(100);
    const component = this;
    this.querySelectorAll('.toggle-processes').forEach(el => {
      const id = el.dataset.for;
      const isExpanded = localStorage.getItem(`ClusterList_expanded_${id}`) === 'true';
      
      if (isExpanded) {
        [...el.children].forEach(child => child.classList.toggle('d-none'));
        const processes = el.parentNode.parentNode.querySelector(`[data-id="${id}"]`);
        processes.classList.remove('d-none');
      }

      el.addEventListener('click', function(e) {
        e.stopPropagation();
        e.preventDefault();
        [...this.children].forEach(el => el.classList.toggle('d-none'));
        const id = this.dataset.for;
        const processes = this.parentNode.parentNode.querySelector(`[data-id="${id}"]`);
        processes.classList.toggle('d-none');
        
        const isExpanded = !processes.classList.contains('d-none');
        localStorage.setItem(`ClusterList_expanded_${id}`, isExpanded);
      });
    });
  }

  render() {
    return html`
      <style>
        #processes-table tbody tr:last-child {
          border-bottom: 1px solid transparent;
        }
      </style>
      <div class="sheet">
        <div class="d-flex justify-content-start align-items-center">
          <i class="bi bi-collection me-3 fs-1"></i>
          <h3 about="v-bpa:ProcessClusters" property="rdfs:label" class="mb-1"></h3>
        </div>

        <!-- Таблица для больших экранов -->
        <div class="d-none d-lg-block">
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
                ${this.clusters?.map(([clusterId]) => html`
                  <tr about="${clusterId}" class="border-top" onclick="location.hash = '#/ClusterView/${clusterId}'">
                    <td class="text-center toggle-processes" style="cursor: pointer;" data-for="${clusterId}">
                      <i class="bi bi-chevron-up text-secondary d-none"></i>
                      <span class="badge bg-success-subtle text-dark rounded-pill">{{this.model['v-bpa:hasProcess']?.length}}</span>
                      <i class="bi bi-chevron-down text-secondary"></i>
                    </td>
                    <td>
                      <p class="mb-0 fw-bold" property="rdfs:label"></p>
                      <p class="mb-0 text-secondary">
                        <${Literal} about="${clusterId}" property="v-bpa:proposedClusterDescription" max-chars="100"></${Literal}>
                      </p>
                    </td>
                    <td>
                      <${Literal} about="${clusterId}" property="v-bpa:clusterSimilarities" max-chars="100"></${Literal}>
                    </td>
                    <td>
                      <${Literal} about="${clusterId}" property="v-bpa:clusterDifferences" max-chars="100"></${Literal}>
                    </td>
                  </tr>
                  <tr about="${clusterId}" class="d-none" data-id="${clusterId}" style="background-color: white!important;">
                    <td></td>
                    <td colspan="3" class="p-0">
                      <div class="table-responsive">
                        <table class="table mb-0 table-hover table-light table-borderless" id="processes-table">
                          <!--thead>
                            <tr>
                              <th width="50%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                              <th width="10%" class="text-secondary fw-normal" about="v-bpa:hasProcessJustification" property="rdfs:label"></th>
                              <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                              <th width="10%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                              <th width="10%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
                            </tr>
                          </thead-->
                          <tbody rel="v-bpa:hasProcess">
                            <tr onclick="location.hash = '#/ProcessView/{{this.model.id}}'">
                              <td class="align-middle">
                                <h5 class="mb-0" property="rdfs:label"></h5>
                                <p class="text-muted mb-0">
                                  <${Literal} about="{{this.model.id}}" property="v-bpa:processDescription" max-chars="100"></${Literal}>
                                </p>
                              </td>
                              <td class="align-middle" rel="v-bpa:hasProcessJustification">
                                <${ProcessJustificationIndicator} class="text-nowrap" about="{{this.model.id}}" property="rdfs:label"></${ProcessJustificationIndicator}>
                              </td>
                              <td class="align-middle" property="v-bpa:responsibleDepartment"></td>
                              <td class="align-middle">
                                <i class="bi bi-people-fill me-1"></i>
                                <strong>{{ this.model.hasValue('v-bpa:processParticipant') && typeof this.model['v-bpa:processParticipant'][0] === 'string' ? this.model['v-bpa:processParticipant'][0].split(',').length : 0 }}</strong>
                              </td>
                              <td class="align-middle lh-sm">
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

        <!-- Карточки для маленьких экранов -->
        <div class="d-lg-none">
          ${this.clusters?.map(([clusterId]) => html`
            <div class="card mb-3" about="${clusterId}">
              <div class="card-body">
                <div class="d-flex align-items-start mb-3" onclick="location.hash = '#/ClusterView/${clusterId}'">
                  <div class="d-flex flex-column align-items-center me-3 toggle-processes" style="cursor: pointer;" data-for="${clusterId}">
                    <i class="bi bi-chevron-up text-secondary d-none"></i>
                    <span class="badge bg-success-subtle text-dark rounded-pill">{{this.model['v-bpa:hasProcess']?.length}}</span>
                    <i class="bi bi-chevron-down text-secondary"></i>
                  </div>
                  <div>
                    <h5 class="card-title mb-1" property="rdfs:label"></h5>
                    <p class="card-text text-secondary small" property="v-bpa:proposedClusterDescription"></p>
                  </div>
                </div>
                <div class="mb-2">
                  <label class="text-secondary small" about="v-bpa:clusterSimilarities" property="rdfs:label"></label>
                  <div property="v-bpa:clusterSimilarities"></div>
                </div>
                <div class="mb-3">
                  <label class="text-secondary small" about="v-bpa:clusterDifferences" property="rdfs:label"></label>
                  <div property="v-bpa:clusterDifferences"></div>
                </div>
                <div class="d-none" rel="v-bpa:hasProcess" data-id="${clusterId}">
                  <div class="card mb-2 bg-light" onclick="location.hash = '#/ProcessView/{{this.model.id}}'">
                    <div class="card-body">
                      <h6 class="mb-1" property="rdfs:label"></h6>
                      <p class="text-muted small mb-0" property="v-bpa:processDescription"></p>
                      <div rel="v-bpa:hasProcessJustification">
                        <${ProcessJustificationIndicator} class="text-nowrap" about="{{this.model.id}}" property="rdfs:label"></${ProcessJustificationIndicator}>
                      </div>
                      <div class="d-flex justify-content-between align-items-center">
                        <div property="v-bpa:responsibleDepartment" class="small"></div>
                        <div class="small">
                          <i class="bi bi-people-fill me-1"></i>
                          {{ this.model.hasValue('v-bpa:processParticipant') && typeof this.model['v-bpa:processParticipant'][0] === 'string' ? this.model['v-bpa:processParticipant'][0].split(',').length : 0 }}
                        </div>
                        <div>
                          <strong>{{ this.model.hasValue('v-bpa:laborCosts') && this.model.hasValue('v-bpa:processFrequency') ? this.model['v-bpa:laborCosts'][0] * this.model['v-bpa:processFrequency'][0] : 0 }}</strong>
                          <small><${Literal} class="text-secondary" about="v-bpa:HoursPerYear" property="rdfs:comment"></${Literal}></small>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          `).join('') ?? ''}
        </div>

      </div>
    `;
  }
}

customElements.define(ClusterList.tag, ClusterList);