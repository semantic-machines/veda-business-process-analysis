import {Component, html, Backend, Model} from 'veda-client';
import BusinessProcessOverview from './BusinessProcessOverview.js';

export default class BusinessProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
    const {id: processIds} = await Backend.stored_query(params);
    const processes = await Promise.all(processIds.map((id) => new Model(id)));
    this.processes = processes;
  }

  async render() {
    return html`
      <div class="sheet">
        <h3>Список бизнес-процессов</h3>
        <hr>
        ${this.processes.map(process => html`
          <${BusinessProcessOverview} about=${process.id}></${BusinessProcessOverview}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(BusinessProcessList.tag, BusinessProcessList);
