import {Component, html} from 'veda-client';
import {Modal} from 'bootstrap';
import ProcessFiltersForm from './ProcessFiltersForm.js';

export default class ProcessFiltersModal extends Component(HTMLElement) {
  static tag = 'bpa-process-filters-modal';

  data = {};

  handleApplyFilters(data) {
    this.data = data;
    this.renderFiltersCount();
    this.dispatchEvent(new CustomEvent('filters-changed', {detail: this.data}));
    Modal.getInstance(this.lastElementChild)?.hide();
  }

  handleResetFilters() {
    this.data = {};
    this.renderFiltersCount();
    this.dispatchEvent(new CustomEvent('filters-changed', {detail: null}));
  }

  render() {
    return html`
      <button type="button" class="btn btn-link text-dark text-decoration-none" data-bs-toggle="modal" data-bs-target="#filters" id="filters-button">
        <i class="bi bi-chevron-down me-1"></i>
        <span about="v-bpa:Filters" property="rdfs:label"></span>
        <span class="badge bg-info ms-1"></span>
      </button>
      <div class="modal fade" id="filters" data-bs-keyboard="true" tabindex="-1" aria-hidden="true">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <div class="modal-header">
              <h1 class="modal-title fs-5" id="staticBackdropLabel" about="v-bpa:Filters" property="rdfs:label"></h1>
              <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body">
              <${ProcessFiltersForm}
                on:apply="${(e) => this.handleApplyFilters(e.detail)}"
                on:reset="${() => this.handleResetFilters()}"
              ></${ProcessFiltersForm}>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  renderFiltersCount() {
    const button = this.querySelector('#filters-button');
    const count = this.data ? Object.values(this.data).filter(value => value.some(v => v)).length || null : null;
    button.lastElementChild.textContent = count ?? '';
  }

  post() {
    this.querySelector('#filters').addEventListener('shown.bs.modal', () => {
      this.querySelector('.btn-close')?.focus();
    });
  }

  removed() {
    Modal.getInstance(this.lastElementChild)?.hide();
  }
}

customElements.define(ProcessFiltersModal.tag, ProcessFiltersModal);
