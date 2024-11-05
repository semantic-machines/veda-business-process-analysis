import {Model, Backend, Component, html} from 'veda-client';

export default class ProcessView extends Component(HTMLElement) {
  static tag = 'bpa-process-view';

  edit() {
    location.hash = `#/ProcessEdit/${this.model.id}`;
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
    return html`
      <div class="sheet">
        <div class="d-flex justify-content-between align-items-center mb-3">
          <div>
            <p class="mb-0 text-muted" about="v-bpa:BusinessProcess" property="rdfs:label"></p>
            <h3>
              <i class="bi bi-diagram-3 me-2"></i>  
              <span property="rdfs:label"></span>&nbsp;
              <span class="fs-5 align-middle" rel="v-bpa:processRelevance">
              ${this.model['v-bpa:processRelevance'][0].id === 'v-bpa:CompletelyJustified' ? html`
                <span class="badge text-bg-success border border-success me-2" property="rdfs:label"></span>
              ` : this.model['v-bpa:processRelevance'][0].id === 'v-bpa:PartlyJustified' ? html`
                <span class="badge text-bg-warning border border-warning me-2" property="rdfs:label"></span>
              ` : html`
                <span class="badge text-bg-danger border border-danger me-2" property="rdfs:label"></span>
                `}
              </span>
            </h3>
          </div>
          <div class="d-flex gap-2">
            ${this.cluster 
              ? html`
                <a href="#/ClusterView/${this.cluster}" style="text-decoration: none;">
                  <div class="card border-0 bg-success p-1 text-dark bg-opacity-10">
                    <div class="card-body p-2">
                      <p class="mb-0 text-muted" about="v-bpa:ProcessCluster" property="rdfs:label"></p>
                      <h5 class="mb-0" >
                        <i class="bi bi-collection me-2"></i>
                        <span about="${this.cluster}" property="rdfs:label"></span>
                      </h5>
                    </div>
                  </div>
                </a>`
              : ''}
          </div>
        </div>
        <hr>
        <div class="row">
          <div class="col-12 col-md-9">
            <p class="mb-0 text-muted" about="v-bpa:processDescription" property="rdfs:label"></p>
            <p class="fw-bold" property="v-bpa:processDescription"></p>

            <p class="mb-0 text-muted" about="v-bpa:responsibleDepartment" property="rdfs:label"></p>
            <p class="fs-6 badge bg-secondary" property="v-bpa:responsibleDepartment"></p>

            <p class="mb-0 text-muted" about="v-bpa:processFrequency" property="rdfs:label"></p>
            <p class="fs-6 badge text-bg-light border border-secondary text-muted">
              <i class="bi bi-arrow-repeat me-1"></i>
              <span property="v-bpa:processFrequency"></span>&nbsp;
              <span about="v-bpa:TimesPerYear" property="rdfs:label"></span>
            </p>

            <p class="mb-0 text-muted" about="v-bpa:laborCosts" property="rdfs:label"></p>
            <p class="fw-bold mb-0">
              <span>${(this.model['v-bpa:laborCosts'][0] * this.model['v-bpa:processFrequency'][0]).toFixed(2)}</span>&nbsp;
              <span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
          <div class="col-12 col-md-3 border-start border-secondary border-opacity-25">
            <p class="mb-0 text-muted" about="v-bpa:processParticipant" property="rdfs:label"></p>
            <p class="fw-bold" property="v-bpa:processParticipant"></p>
          </div>
        </div>
      </div>
      <div class="sheet">
        <h4 about="v-bpa:ProcessDocument" property="rdfs:label"></h4>
      </div>
      <button @click="edit" class="btn btn-primary mb-3">
        <span about="v-bpa:Edit" property="rdfs:label"></span>
      </button>
    `;
  }
}

customElements.define(ProcessView.tag, ProcessView);
