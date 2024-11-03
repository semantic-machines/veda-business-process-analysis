export function toTurtle(model) {
  return Object.entries(model).map(([predicate, objects]) => {
    if (!Array.isArray(objects)) return '';
      return objects.map(obj => {
        if (typeof obj === 'object' && obj.id) {
          return `  ${predicate} <b>${obj.id}</b> ;`;
        } else if (typeof obj === 'string') {
          return `  ${predicate} "${obj}" ;`; 
        } else {
          return `  ${predicate} ${obj} ;`;
        }
      }).join('\n');
    }).filter(Boolean).join('\n');
}