import {Component, html} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator';
import Literal from './Literal';

export default class ClusterView extends Component(HTMLElement) {
  static tag = 'bpa-cluster-view';

  showSimilarProcesses = true;
  
  toggleShowSimilarProcesses() {
    this.showSimilarProcesses = !this.showSimilarProcesses;
    this.update();
  }

  render() {
    const estimatedLaborCost = this.model['v-bpa:estimatedLaborCost']?.[0];
    const proposedFrequency = this.model['v-bpa:proposedFrequency']?.[0];

    return html`
      <div class="sheet">
        <div class="d-flex justify-content-between align-items-center mb-2">
          <div>
            <p class="mb-0 text-muted" about="v-bpa:ProcessCluster" property="rdfs:label"></p>
            <h3 class="mb-0">
              <i class="bi bi-collection me-2"></i>
              <span property="rdfs:label"></span> 
            </h3>
          </div>
        </div>
        <p class="mb-0 text-muted" about="v-bpa:clusterReason" property="rdfs:label"></p>
        <p property="v-bpa:clusterReason"></p>
        <hr>
        <div class="row g-5">
          <div class="col-12 col-md-6 mb-3 border-end border-secondary border-opacity-25">
            <i class="bi bi-intersect fs-2 float-start me-3"></i>
            <p class="mb-0 text-muted" about="v-bpa:clusterSimilarities" property="rdfs:label"></p>
            <p class="mb-0" property="v-bpa:clusterSimilarities"></p>
          </div>
          <div class="col-12 col-md-6 mb-3">
            <i class="bi bi-exclude fs-2 float-start me-3"></i>
            <p class="mb-0 text-muted" about="v-bpa:clusterDifferences" property="rdfs:label"></p>
            <p class="mb-0" property="v-bpa:clusterDifferences"></p>
          </div>
        </div>
      </div>
      
      <ul class="nav nav-underline mx-3 mb-2">
        <li class="nav-item">
          <button @click="toggleShowSimilarProcesses" class="nav-link ${this.showSimilarProcesses ? 'active disabled' : 'text-secondary-emphasis'}">
            <span class="me-2" about="v-bpa:SimilarProcesses" property="rdfs:label"></span>
            <span class="align-top badge rounded-pill bg-secondary">${this.model.hasValue('v-bpa:hasProcess') ? this.model['v-bpa:hasProcess'].length : 0}</span>
          </button>
        </li>
        <li class="nav-item">
          <button @click="toggleShowSimilarProcesses" class="nav-link ${!this.showSimilarProcesses ? 'active disabled' : 'text-secondary-emphasis'}">
            <span class="me-2" about="v-bpa:ProposedProcess" property="rdfs:label"></span>
          </button>
        </li>
      </ul>
      
      ${this.showSimilarProcesses 
        ? html`
          <div class="sheet">
            <div class="table-responsive">
              <style>
                #processes-table > tbody > tr:last-child {
                  border-bottom: 1px solid transparent;
                }
              </style>
              <table class="table mb-0 table-hover" id="processes-table">
                <thead>
                  <tr>
                    <th width="50%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                    <th width="10%" class="text-secondary fw-normal" about="v-bpa:hasProcessJustification" property="rdfs:label"></th>
                    <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                    <th width="10%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                    <th width="10%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
                  </tr>
                </thead>
                <tbody rel="v-bpa:hasProcess">
                  <tr about="{{this.model.id}}" class="process-row" onclick="location.href = '#/ProcessView/{{this.model.id}}'">
                    <td class="align-middle">
                      <h5 class="mb-0" property="rdfs:label"></h5>
                      <p class="text-muted mb-0">
                        <${Literal} about="{{this.model.id}}" property="v-bpa:processDescription" max-chars="70"></${Literal}>
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
          </div>`
        : html`
          <div class="sheet">
            <div class="row">
              <div class="col-12 col-md-8">
                <div class="mb-3">
                  <p class="mb-0 text-muted" about="v-bpa:BusinessProcess" property="rdfs:label"></p>
                  <h3 class="mb-0">
                    <i class="bi bi-diagram-3 me-2"></i>
                    <span class="me-3" property="v-bpa:proposedClusterName"></span>
                  </h3>
                  <${ProcessJustificationIndicator} about="v-bpa:CompletelyJustified" property="rdfs:comment"></${ProcessJustificationIndicator}>
                </div>
                <p class="mb-0 text-muted" about="v-bpa:processDescription" property="rdfs:label"></p>
                <p property="v-bpa:proposedClusterDescription"></p>
              </div>
              <div class="col-12 col-md-4">
                <div class="accordion" id="ProcessViewAccordion">
                  <style>
                    #ProcessViewAccordion .accordion-button:after {
                      margin-left: 0.5em;
                    }
                  </style>
                  <div class="accordion-item" style="padding:1rem 1.25rem;>
                    <h5 class="accordion-header">
                      <p class="mb-0 text-muted" about="v-bpa:responsibleDepartment" property="rdfs:comment"></p>
                      <p class="mb-0 fw-bold" property="v-bpa:proposedDepartment"></p>
                    </h5>
                  </div>
                  <div class="accordion-item border-bottom-0">
                    <h2 class="accordion-header">
                      <button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target="#collapseTwo" aria-expanded="false" aria-controls="collapseTwo">
                        <div class="me-auto fw-bold" about="v-bpa:processParticipant" property="rdfs:comment"></div>
                        <div class="ms-auto">
                          <i class="bi bi-people-fill me-1"></i>
                          ${this.model.hasValue('v-bpa:proposedParticipants') ? this.model['v-bpa:proposedParticipants']?.[0].split(',').length : 0}
                        </div>
                      </button>
                    </h2>
                    <div id="collapseTwo" class="accordion-collapse collapse" data-bs-parent="#accordionExample">
                      <div class="accordion-body">
                        <div property="v-bpa:proposedParticipants"></div>
                      </div>
                    </div>
                  </div>
                  <div class="accordion-item">
                    <h2 class="accordion-header">
                      <button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target="#collapseOne" aria-expanded="false" aria-controls="collapseOne">
                        <div class="me-auto fw-bold" about="v-bpa:TotalTimeEffort" property="rdfs:comment"></div>
                        <div class="ms-auto">
                          ${estimatedLaborCost && proposedFrequency ? (estimatedLaborCost * proposedFrequency).toFixed(2) : '0.00'}&nbsp;
                          <span about="v-bpa:Hours" property="rdfs:comment"></span>
                        </div>
                      </button>
                    </h2>
                    <div id="collapseOne" class="accordion-collapse collapse" data-bs-parent="#accordionExample">
                      <div class="accordion-body">
                        <div class="d-flex justify-content-between">
                          <div class="text-secondary" about="v-bpa:laborCosts" property="rdfs:label"></div>
                          <div><span property="v-bpa:estimatedLaborCost"></span>&nbsp;<span about="v-bpa:Hours" property="rdfs:comment"></span></div>
                        </div>
                        <div class="d-flex justify-content-between">
                          <div class="text-secondary" about="v-bpa:processFrequency" property="rdfs:comment"></div>
                          <div><span property="v-bpa:proposedFrequency"></span>&nbsp;<span about="v-bpa:TimesPerYear" property="rdfs:label"></span></div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        `
      }
    `;
  }
}

customElements.define(ClusterView.tag, ClusterView);
