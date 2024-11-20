import {Model, Backend, Component, html} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Literal from './Literal.js';

export default class DocumentView extends Component(HTMLElement) {
  static tag = 'bpa-document-view';

  edit() {
    location.hash = `#/DocumentEdit/${this.model.id}`;
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
    params['v-s:storedQuery'] = 'v-bpa:DocumentInProcess';
    params['v-bpa:hasProcessDocument'] = this.model.id;
    params['v-s:resultFormat'] = 'rows';
    const {rows: processes} = await Backend.stored_query(params);
    this.processes = processes;
  }

  render() {
    return html`
      <div class="sheet">
        <div class="mb-3">
          <p class="mb-0 text-muted" about="v-bpa:ProcessDocument" property="rdfs:label"></p>
          <h3 class="mb-0">
            <i class="bi bi-file-earmark-text me-2"></i>
            <span class="me-3" property="v-bpa:documentName"></span>
          </h3>
        </div>
        <p class="mb-0 text-muted" about="v-bpa:documentContent" property="rdfs:label"></p>
        <p property="v-bpa:documentContent"></p>
      </div>
      
      <div class="sheet">
        <div class="table-responsive">
          <style>
            #processes-table tbody tr:last-child {
              border-bottom: 1px solid transparent;
            }
          </style>
          <table class="table table-hover mb-0" id="processes-table">
            <thead>
              <tr>
                <th width="50%" class="text-secondary fw-normal" about="v-bpa:BusinessProcess" property="rdfs:label"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:hasProcessJustification" property="rdfs:label"></th>
                <th width="20%" class="text-secondary fw-normal" about="v-bpa:responsibleDepartment" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal" about="v-bpa:processParticipant" property="rdfs:comment"></th>
                <th width="10%" class="text-secondary fw-normal"><span about="v-bpa:laborCosts" property="rdfs:label"></span></th>
              </tr>
            </thead>
            <tbody>
              ${this.processes.map(([id, label, description, justification, responsibleDepartment, processParticipant, laborCosts]) => html`
                <tr onclick="location.hash = '#/ProcessView/${id}'">
                  <td class="align-middle"><h5 class="mb-0">${label}</h5><p class="text-muted mb-0">${description && description.length > 60 ? description.slice(0, 60) + '...' : description}</p></td>
                  <td class="align-middle"><${ProcessJustificationIndicator} class="text-nowrap" about="${justification}" property="rdfs:label"></${ProcessJustificationIndicator}></td>
                  <td class="align-middle">${responsibleDepartment}</td>
                  <td class="align-middle">
                    <i class="bi bi-people-fill me-1"></i>
                    <strong>${processParticipant && typeof processParticipant === 'string' ? processParticipant.split(',').length : 0}</strong>
                  </td>
                  <td class="align-middle lh-sm">
                    <strong>${laborCosts ?? 0}</strong><br>
                    <small><${Literal} class="text-secondary" about="v-bpa:HoursPerYear" property="rdfs:comment"></${Literal}></small>
                  </td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
      
      <div class="d-flex justify-content-start gap-2 mt-3">
        <button @click="${(e) => this.edit(e)}" class="btn btn-primary">
          <span about="v-bpa:Edit" property="rdfs:label"></span>
        </button>
        <button @click="${(e) => this.remove(e)}" class="btn btn-link text-muted text-decoration-none">
          <span about="v-bpa:Remove" property="rdfs:label"></span>
        </button>
      </div>
    `;
  }
}

customElements.define(DocumentView.tag, DocumentView);
