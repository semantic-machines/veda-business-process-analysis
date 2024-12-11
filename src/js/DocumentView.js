import {Model, Backend, Component, html, safe} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';
import Literal from './Literal.js';

export default class DocumentView extends Component(HTMLElement) {
  static tag = 'bpa-document-view';

  edit() {
    location.hash = `#/DocumentEdit/${this.model.id}`;
  }

  async remove() {
    if (confirm('Вы уверены?')) {
      try {
        await Promise.all(this.processes.map(async ([id]) => {
          const process = await new Model(id).load();
          process.removeValue('v-bpa:hasProcessDocument', this.model.id);
          await process.save();
        }));
        await this.model.remove();
        location.hash = '#/ProcessOverview';
      } catch (error) {
        console.error(error);
        alert('Ошибка при удалении документа');
      }
    }
  }

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:DocumentInProcess';
    params['v-bpa:hasProcessDocument'] = this.model.id;
    params['v-s:resultFormat'] = 'rows';
    try {
      const {rows: processes} = await Backend.stored_query(params);
      this.processes = processes;
    } catch (e) {
      console.error('Ошибка при запросе процессов документа', e);
      this.processes = [];
    }
  }

  render() {
    return html`
      <div class="sheet">
        <div class="row">
          <div class="col-sm-9">
            <h3>
              <span class="me-3" property="v-bpa:documentTitle"></span>
            </h3>
            <div rel="v-s:attachment" class="mb-3">
              <a class="text-secondary" href="/files/{{this.model.id}}" target="_blank">
                <i class="bi bi-cloud-download me-2"></i><span class="me-3" property="v-s:fileName"></span>
              </a>
            </div>
            ${this.model['v-bpa:hasDocumentSection']?.map(section => html`
              <div about="${section.id}">
                <h4 property="v-bpa:sectionTitle"></h4>
                <p property="v-bpa:sectionContent" style="white-space:pre-line;"></p>
              </div>
            `).join('')}
          </div>
          <div class="col-sm-3">
            <div class="accordion" id="DocumentViewAccordion">
              <style>
                #DocumentViewAccordion .accordion-button:after {
                  margin-left: 0.5em;
                }
                #DocumentViewAccordion .accordion-item {
                  border-color: #aaa;
                }
              </style>
              <div class="accordion-item" style="padding:1rem 1.25rem;">
                <div class="accordion-header mb-3">
                  <p class="mb-0 text-muted" about="v-bpa:documentType" property="rdfs:comment"></p>
                  <p class="mb-0" property="v-bpa:documentType"></p>
                </div>
                <div class="accordion-header">
                  <p class="mb-0 text-muted" about="v-bpa:documentDepartment" property="rdfs:label"></p>
                  <p class="mb-0" rel="v-bpa:hasDepartment"><span property="rdfs:label"></span></p>
                </div>
              </div>
              <div class="accordion-item">
                <h2 class="accordion-header">
                  <button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target="#collapse1" aria-expanded="false" aria-controls="collapse1">
                    <div class="me-3 fw-bold" about="v-bpa:documentSource" property="rdfs:comment"></div>
                    <div class="ms-auto">
                      <span property="v-bpa:documentSource"></span>
                    </div>
                  </button>
                </h2>
                <div id="collapse1" class="accordion-collapse collapse" data-bs-parent="#accordionExample">
                  <div class="accordion-body">
                    <div class="d-flex justify-content-between">
                      <div class="text-secondary" about="v-bpa:documentSignedBy" property="rdfs:label"></div>
                      <div><span property="v-bpa:documentSignedBy"></span></div>
                    </div>
                    <div class="d-flex justify-content-between">
                      <div class="text-secondary" about="v-bpa:documentSignedDate" property="rdfs:comment"></div>
                      <div>
                        ${Date.parse(this.model['v-bpa:documentSignedDate']?.[0])
                          ? new Date(this.model['v-bpa:documentSignedDate']?.[0]).toLocaleDateString('ru-RU')
                          : this.model['v-bpa:documentSignedDate']?.[0] ?? ''
                        }
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            ${this.processes && this.processes.length > 0
              ? this.processes.map(([id, label, description, justification, department, participant, timeEffort]) => html`
              <a href="#/ProcessView/${id}" class="text-decoration-none d-block text-dark">
                <div class="card border-0 bg-secondary bg-opacity-10 mt-3">
                  <div class="card-body">
                    <div class="d-flex justify-content-between align-items-center gap-2">
                      <div>
                        <p class="mb-0 text-muted" about="v-bpa:BusinessProcess" property="rdfs:label"></p>
                        <h5 class="mb-0">
                          <i class="bi bi-gear me-2"></i>
                          <span>${label}</span>
                        </h5>
                      </div>
                      <div class="d-flex align-items-center gap-3">
                        <i class="bi bi-chevron-right align-bottom mt-1 fs-5"></i>
                      </div>
                    </div>
                  </div>
                </div>
              </a>`
              ).join('')
              : ''
            }

            <div class="d-flex justify-content-start gap-2 mt-3">
              <!--button on:click="${(e) => this.edit(e)}" class="btn btn-primary">
                <span about="v-bpa:Edit" property="rdfs:label"></span>
              </button-->
              <button on:click="${(e) => this.remove(e)}" class="btn btn-link text-muted text-decoration-none">
                <span about="v-bpa:Remove" property="rdfs:label"></span>
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}

customElements.define(DocumentView.tag, DocumentView);
