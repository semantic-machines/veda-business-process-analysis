import {Component, html, Backend, Model} from 'veda-client';

export default class DocumentList extends Component(HTMLElement) {
  static tag = 'bpa-document-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessDocuments';
    params['v-s:resultFormat'] = 'rows';
    const {rows: documents} = await Backend.stored_query(params);
    this.documents = documents;
    this.filtersData = null;
    this.filtered = this.documents;
  }

  goToDocument(event) {
    const id = event.target.closest('tr').dataset.about;
    location.hash = `#/DocumentView/${id}`;
  }

  applyFilters(event) {
    event.preventDefault();
    const form = event.target.closest('form');
    const formData = new FormData(form);
    const data = Object.fromEntries(formData.entries());
    this.filtersData = data;

    this.filtered = this.documents.filter(([id, name, content, created]) => {
      // Фильтр по названию
      if (data['v-bpa:documentName'] && !name.toLowerCase().includes(data['v-bpa:documentName'].toLowerCase())) {
        return false;
      }
      // Фильтр по содержанию
      if (data['v-bpa:documentContent'] && 
          !content.toLowerCase().includes(data['v-bpa:documentContent'].toLowerCase())) {
        return false;
      }
      // Фильтр по дате создания
      created = new Date(created);
      if (data.createdFrom) {
        const from = new Date(data.createdFrom);
        from.setHours(0, 0, 0, 0);
        if (created < from) return false;
      }
      if (data.createdTo) {
        const to = new Date(data.createdTo); 
        to.setHours(23, 59, 59, 999);
        if (created > to) return false;
      }
      return true;
    });

    this.renderFilteredDocuments();
    this.renderFiltersCount();
  }

  resetFilters() {
    this.filtersData = null;
    this.filtered = this.documents;
  }

  renderFilteredDocuments() {
    const container = this.querySelector('#filtered-documents');
    container.innerHTML = `
      ${this.filtered.map(([id, name, content, created]) => html`
        <tr onclick="location.hash = '#/DocumentView/${id}'">
          <td class="align-middle"><h5 class="mb-0">${name}</h5><p class="text-muted mb-0">${content && content.length > 120 ? content.slice(0, 120) + '...' : content}</p></td>
          <td class="align-middle text-end">${new Date(created).toLocaleDateString()}</td>
        </tr>
      `).join('')}
    `;
  }

  renderFiltersCount() {
    const button = this.querySelector('#filters-button');
    const count = this.filtersData ? Object.values(this.filtersData).filter(value => value).length || null : null;
    button.lastElementChild.textContent = count ?? '';
  }

  post() {
    this.renderFilteredDocuments();
  }

  render() {
    return html`
      <div class="sheet">
        <div class="d-flex align-items-center">
          <i class="bi bi-file-earmark-text ms-2 me-3 fs-1"></i>
          <h3 class="mb-1" about="v-bpa:ProcessDocuments" property="rdfs:label"></h3>
          <button type="button" class="btn btn-link text-dark text-decoration-none ms-auto" data-bs-toggle="modal" data-bs-target="#filters" id="filters-button">
            <i class="bi bi-chevron-down me-1"></i>
            <span about="v-bpa:Filters" property="rdfs:label"></span>
            <span class="badge rounded-pill bg-danger ms-1"></span>
          </button>
        </div>
        <div class="table-responsive">
          <style>
            #processes-table tbody tr:last-child {
              border-bottom: 1px solid transparent;
            }
          </style>
          <table class="table table-hover mb-0" id="processes-table">
            <thead>
              <tr>
                <th width="100%" class="text-secondary fw-normal" about="v-bpa:documentName" property="rdfs:label"></th>
                <th class="text-secondary fw-normal text-end text-nowrap" about="v-s:created" property="rdfs:label"></th>
              </tr>
            </thead>
            <tbody id="filtered-documents"></tbody>
          </table>
          <div class="modal" id="filters" data-bs-backdrop="static" data-bs-keyboard="false" tabindex="-1" aria-labelledby="staticBackdropLabel" aria-hidden="true">
            <div class="modal-dialog modal-dialog-centered">
              <div class="modal-content">
                <div class="modal-header">
                  <h1 class="modal-title fs-5" id="staticBackdropLabel" about="v-bpa:Filters" property="rdfs:label"></h1>
                  <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                </div>
                <div class="modal-body">
                  <form @submit="${(e) => this.applyFilters(e)}">
                    <div class="mb-5">
                      <div class="mb-3">
                        <label for="name" class="form-label" about="v-bpa:documentName" property="rdfs:label"></label>
                        <input type="text" class="form-control" id="name" name="v-bpa:documentName">
                      </div>
                      <div class="mb-3">
                        <label for="content" class="form-label" about="v-bpa:documentContent" property="rdfs:label"></label>
                        <input type="text" class="form-control" id="content" name="v-bpa:documentContent">
                      </div>
                      <div class="mb-3">
                        <label class="form-label me-2" about="v-s:created" property="rdfs:label"></label>
                        <div class="mb-3 d-flex align-items-center" id="created">
                          <input type="date" placeholder="от" class="form-control me-2 w-50" name="createdFrom">
                          <input type="date" placeholder="до" class="form-control w-50" name="createdTo">
                        </div>
                      </div>
                    </div>
                    <button type="submit" class="btn btn-secondary me-2" data-bs-dismiss="modal"><span about="v-bpa:ApplyFilters" property="rdfs:label"></span></button>
                    <button type="reset" @click="${(e) => this.resetFilters(e)}" class="btn btn-light"><span about="v-bpa:ResetFilters" property="rdfs:label"></span></button>
                  </form>
                </div>
              </div>
            </div>
          </div>        
        </div>
      </div>
    `;
  }
}

customElements.define(DocumentList.tag, DocumentList);