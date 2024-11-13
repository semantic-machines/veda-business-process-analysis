import {Model, Backend, Component, html} from 'veda-client';
import ProcessRelevanceIndicator from './ProcessRelevanceIndicator';

export default class ProcessView extends Component(HTMLElement) {
  static tag = 'bpa-process-view';

  edit() {
    location.hash = `#/ProcessEdit/${this.model.id}`;
  }

  async remove() {
    if (confirm('Вы уверены?')) {
      await this.model.remove();
      location.hash = '#/ProcessOverview';
    }
  }
  
  async added() {
    const params = new Model; 
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:ProcessInClusters';
    params['v-bpa:hasProcess'] = this.model.id;
    const {id: [cluster]} = await Backend.stored_query(params);
    this.cluster = cluster;
  }
  
  render() {
    const laborCosts = this.model['v-bpa:laborCosts']?.[0];
    const processFrequency = this.model['v-bpa:processFrequency']?.[0];

    return html`
      <div class="sheet">
        <div class="row">
          <div class="col-12 col-md-8">
            <div class="mb-3">
              <p class="mb-0 text-muted" about="v-bpa:BusinessProcess" property="rdfs:label"></p>
              <h3 class="mb-0">
                <i class="bi bi-diagram-3 me-2"></i>
                <span class="me-3" property="rdfs:label"></span>
              </h3>
              <span class="me-2 align-middle" rel="v-bpa:processRelevance">
                <${ProcessRelevanceIndicator} about="{{this.model.id}}" property="rdfs:comment"></${ProcessRelevanceIndicator}>
              </span>
            </div>
            <p class="mb-0 text-muted" about="v-bpa:processDescription" property="rdfs:label"></p>
            <p property="v-bpa:processDescription"></p>
          </div>
          <div class="col-12 col-md-4">
            <p class="mb-0 text-muted" about="v-bpa:responsibleDepartment" property="rdfs:comment"></p>
            <p property="v-bpa:responsibleDepartment"></p>

            <div class="accordion" id="ProcessViewAccordion">
              <style>
                #ProcessViewAccordion .accordion-button:after {
                  margin-left: 0.5em;
                }
              </style>
              <div class="accordion-item">
                <h2 class="accordion-header">
                  <button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target="#collapseOne" aria-expanded="false" aria-controls="collapseOne">
                    <div class="me-auto fw-bold" about="v-bpa:TotalTimeEffort" property="rdfs:comment"></div>
                    <div class="ms-auto">
                      ${laborCosts && processFrequency ? (laborCosts * processFrequency).toFixed(2) : '0.00'}&nbsp;
                      <span about="v-bpa:Hours" property="rdfs:comment"></span>
                    </div>
                  </button>
                </h2>
                <div id="collapseOne" class="accordion-collapse collapse" data-bs-parent="#accordionExample">
                  <div class="accordion-body">
                    <div class="d-flex justify-content-between">
                      <div class="text-secondary" about="v-bpa:laborCosts" property="rdfs:label"></div>
                      <div><span property="v-bpa:laborCosts"></span>&nbsp;<span about="v-bpa:Hours" property="rdfs:comment"></span></div>
                    </div>
                    <div class="d-flex justify-content-between">
                      <div class="text-secondary" about="v-bpa:processFrequency" property="rdfs:comment"></div>
                      <div><span property="v-bpa:processFrequency"></span>&nbsp;<span about="v-bpa:TimesPerYear" property="rdfs:label"></span></div>
                    </div>
                  </div>
                </div>
              </div>
              <div class="accordion-item">
                <h2 class="accordion-header">
                  <button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target="#collapseTwo" aria-expanded="false" aria-controls="collapseTwo">
                    <div class="me-auto fw-bold" about="v-bpa:processParticipant" property="rdfs:comment"></div>
                    <div class="ms-auto">
                      <i class="bi bi-people-fill me-1"></i>
                      ${this.model.hasValue('v-bpa:processParticipant') ? this.model['v-bpa:processParticipant']?.[0].split(',').length : 0}
                    </div>
                  </button>
                </h2>
                <div id="collapseTwo" class="accordion-collapse collapse" data-bs-parent="#accordionExample">
                  <div class="accordion-body">
                    <div property="v-bpa:processParticipant"></div>
                  </div>
                </div>
              </div>
            </div>

            <div class="d-flex gap-2">
              ${this.cluster 
                ? html`
                  <a href="#/ClusterView/${this.cluster}" style="text-decoration: none;">
                    <div class="card border-0 bg-success p-1 text-dark bg-opacity-10">
                      <div class="card-body p-2">
                        <p class="mb-0 text-muted" about="v-bpa:ProcessCluster" property="rdfs:label"></p>
                        <h5 class="mb-0">
                          <i class="bi bi-collection me-2"></i>
                          <span about="${this.cluster}" property="rdfs:label"></span>
                        </h5>
                      </div>
                    </div>
                  </a>`
                : ''}
            </div>
          </div>
        </div>
      </div>
      <div class="sheet">
        <h4 about="v-bpa:ProcessDocument" property="rdfs:label"></h4>
      </div>
      <div class="d-flex justify-content-start gap-2 mt-3">
        <button @click="edit" class="btn btn-primary">
          <span about="v-bpa:Edit" property="rdfs:label"></span>
        </button>
        <button @click="remove" class="btn btn-link text-muted text-decoration-none">
          <span about="v-bpa:Remove" property="rdfs:label"></span>
        </button>
      </div>
    `;
  }
}

customElements.define(ProcessView.tag, ProcessView);
