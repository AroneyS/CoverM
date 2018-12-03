use std;

use coverage_takers::*;
use ReadsMapped;

pub enum CoveragePrinter {
    StreamedCoveragePrinter,
    SparseCachedCoveragePrinter,
    DenseCachedCoveragePrinter {
        entry_type: Option<String>,
        estimator_headers: Option<Vec<String>>,
    },
}

impl CoveragePrinter {
    pub fn finalise_printing<'a>(
        &mut self,
        cached_coverage_taker: &'a CoverageTakerType<'a>,
        print_stream: &mut std::io::Write,
        reads_mapped_per_sample: Option<&Vec<ReadsMapped>>,
        columns_to_normalise: &Vec<usize>) {
        match self {
            CoveragePrinter::StreamedCoveragePrinter => {},
            CoveragePrinter::SparseCachedCoveragePrinter => {
                print_sparse_cached_coverage_taker(
                    cached_coverage_taker, print_stream, reads_mapped_per_sample,
                    &columns_to_normalise);
            },
            CoveragePrinter::DenseCachedCoveragePrinter {
                entry_type,
                estimator_headers
            } => {
                print_dense_cached_coverage_taker(
                    &(entry_type.as_ref().unwrap()), estimator_headers.as_ref().unwrap(),
                    cached_coverage_taker, print_stream, reads_mapped_per_sample,
                    &columns_to_normalise);
            }
        }
    }

    pub fn print_headers(
        &mut self,
        entry_type_str: &str,
        estimator_headers_vec: Vec<String>,
        print_stream: &mut std::io::Write) {

        match self {
            CoveragePrinter::StreamedCoveragePrinter |
            CoveragePrinter::SparseCachedCoveragePrinter => {
                write!(print_stream, "Sample\t{}", entry_type_str).unwrap();
                for h in estimator_headers_vec {
                    write!(print_stream, "\t{}", h).unwrap();
                }
                writeln!(print_stream).unwrap();
            },
            CoveragePrinter::DenseCachedCoveragePrinter {
                ref mut entry_type,
                ref mut estimator_headers
            } => {
                *entry_type = Some(entry_type_str.to_string());
                *estimator_headers = Some(estimator_headers_vec.iter().map(
                    |s| s.to_string()).collect());
            }
        }
    }
}




pub fn print_sparse_cached_coverage_taker<'a>(
    cached_coverage_taker: &'a CoverageTakerType<'a>,
    print_stream: &mut std::io::Write,
    reads_mapped_per_sample: Option<&Vec<ReadsMapped>>,
    columns_to_normalise: &Vec<usize>) {

    let iterator = cached_coverage_taker.generate_iterator();

    match &cached_coverage_taker {
        CoverageTakerType::CachedSingleFloatCoverageTaker{
            stoit_names,
            ref entry_names,
            coverages:_,
            current_stoit_index:_,
            current_entry_index:_,
            num_coverages
        } => {
            debug!("Generating iterator for cached coverage taker with stoit names {:?},\
                    entry_names {:?}\
                    num_coverages {}",
                   stoit_names, entry_names, num_coverages);
            // Print the relative abundance of each genome, with an
            // 'unmapped' entry for reads that don't map.
            let mut current_stoit_coverages: Vec<Vec<f32>> = vec!();
            let mut current_stoit_entry_indices: Vec<usize> = vec!();
            let mut current_stoit_index = 0;

            let mut print_previous_stoit = |
            current_stoit_coverages: &Vec<Vec<f32>>,
            current_stoit_entry_indices: &Vec<usize>,
            current_stoit_index: usize| {
                let mut coverage_multipliers: Vec<Option<f32>> = vec!(None; *num_coverages);
                let mut coverage_totals: Vec<Option<f32>> = vec!(None; *num_coverages);

                // Calculate totals and multipliers for each normalised sample.
                for i in columns_to_normalise {
                    let mut total_coverage = 0.0;
                    for coverage_set in current_stoit_coverages {
                        total_coverage += coverage_set[*i]
                    }
                    coverage_totals[*i] = Some(total_coverage);

                    if reads_mapped_per_sample.is_some() {
                        let reads_mapped = &reads_mapped_per_sample
                            .as_ref().unwrap()[current_stoit_index];
                        let fraction_mapped = reads_mapped.num_mapped_reads as f32 /
                            reads_mapped.num_reads as f32;
                        coverage_multipliers[*i] = Some(fraction_mapped);
                    }
                }

                // Print unmapped entries at the top
                let stoit = &stoit_names[current_stoit_index];
                if reads_mapped_per_sample.is_some() {
                    write!(print_stream, "{}\tunmapped", stoit).unwrap();
                    for (i, column) in columns_to_normalise.iter().enumerate() {
                        if i == 0 {
                            for _ in 0..*column {
                                write!(print_stream, "\tNA").unwrap();
                            }
                        } else {
                            for _ in (columns_to_normalise[i-1]+1)..*column {
                                write!(print_stream, "\tNA").unwrap();
                            }
                        }
                        write!(print_stream, "\t{}",
                               100.0*(1.0-(coverage_multipliers[*column].unwrap())));
                    }
                    for _ in (columns_to_normalise[columns_to_normalise.len()-1]+1)
                        ..*num_coverages {
                            write!(print_stream, "\tNA").unwrap();
                        }
                    writeln!(print_stream).unwrap();
                }

                // Print the actual coverage values
                for (entry_i, coverages) in
                    current_stoit_entry_indices.iter().zip(current_stoit_coverages.iter()) {
                        write!(print_stream, "{}\t{}",
                                 stoit,
                                 match &entry_names[*entry_i] {
                                     Some(s) => s,
                                     None => panic!("Didn't find entry name string as expected")
                                 }).unwrap();

                        for i in 0..*num_coverages {
                            if columns_to_normalise.contains(&i) {
                                write!(
                                    print_stream, "\t{}",
                                    coverages[i]
                                        *100.0
                                        *coverage_multipliers[i].unwrap()
                                        /coverage_totals[i].unwrap()).unwrap();
                            } else {
                                write!(print_stream, "\t{}",
                                       coverages[i]).unwrap();
                            }
                        }
                        writeln!(print_stream);
                    }
            };
            for entry_and_coverages in iterator {
                if current_stoit_index != entry_and_coverages.stoit_index {
                    print_previous_stoit(
                        &current_stoit_coverages,
                        &current_stoit_entry_indices,
                        current_stoit_index);
                    current_stoit_coverages = vec!();
                    current_stoit_entry_indices = vec!();
                    current_stoit_index = entry_and_coverages.stoit_index;
                }
                current_stoit_coverages.push(entry_and_coverages.coverages);
                current_stoit_entry_indices.push(entry_and_coverages.entry_index);
            }
            print_previous_stoit(
                &current_stoit_coverages,
                &current_stoit_entry_indices,
                current_stoit_index);
        },
        _ => panic!("programming error")
    }
}


pub fn print_dense_cached_coverage_taker<'a>(
    entry_type: &str,
    estimator_headers: &Vec<String>,
    cached_coverage_taker: &'a CoverageTakerType<'a>,
    print_stream: &mut std::io::Write,
    reads_mapped_per_sample: Option<&Vec<ReadsMapped>>,
    columns_to_normalise: &Vec<usize>) {

    match &cached_coverage_taker {
        CoverageTakerType::CachedSingleFloatCoverageTaker{
            stoit_names,
            entry_names,
            coverages:_,
            current_stoit_index:_,
            current_entry_index:_,
            num_coverages
        } => {
            debug!("Generating iterator for cached coverage taker with stoit names {:?},\
                    entry_names {:?}\
                    num_coverages {}",
                   stoit_names, entry_names, num_coverages);

            // Print headers
            write!(print_stream, "{}", entry_type).unwrap();
            for stoit_name in stoit_names {
                for estimator_header in estimator_headers {
                    write!(print_stream, "\t{} {}",
                           stoit_name, estimator_header).unwrap();
                }
            }
            writeln!(print_stream).unwrap();

            // There is a coverage multiplier for each stoit
            let mut coverage_multipliers: Vec<f32> =
                match reads_mapped_per_sample {
                    Some(rm) => rm.iter().map(
                        |r| r.num_mapped_reads as f32 / r.num_reads as f32)
                        .collect(),
                    None => vec!()
                };

            // Print unmapped entries at the top if needed
            let mut stoit_by_entry_by_coverage: Vec<Vec<EntryAndCoverages>> = vec!();
            if reads_mapped_per_sample.is_some() {
                write!(print_stream, "unmapped").unwrap();
                for (stoit_i, _) in stoit_names.iter().enumerate() {
                    for (i, column) in columns_to_normalise.iter().enumerate() {
                        if i == 0 {
                            for _ in 0..*column {
                                write!(print_stream, "\tNA").unwrap();
                            }
                        } else {
                            for _ in (columns_to_normalise[i-1]+1)..*column {
                                write!(print_stream, "\tNA").unwrap();
                            }
                        }
                        write!(print_stream, "\t{}",
                               100.0*(1.0-(coverage_multipliers[stoit_i])));
                    }
                    for _ in (columns_to_normalise[columns_to_normalise.len()-1]+1)
                        ..*num_coverages {
                            write!(print_stream, "\tNA").unwrap();
                        }
                }
                writeln!(print_stream).unwrap();
            }

            // Iterate over all entries for one stoit, and then entries for the next,
            // etc. Since we print all entries for each stoit first, because I'm too
            // lazy to re-write the iterator to be stoit by entry.
            let iterator = cached_coverage_taker.generate_iterator();
            // Coverage total for each stoit for each coverage type
            let mut coverage_totals: Vec<Vec<Option<f32>>> = vec![
                vec!(None; *num_coverages); stoit_names.len()];
            for ecs in iterator {
                if reads_mapped_per_sample.is_some() {
                    for i in columns_to_normalise {
                        coverage_totals[ecs.stoit_index as usize][*i] =
                            match coverage_totals[ecs.stoit_index as usize][*i] {
                                Some(total) => {
                                    Some(total + ecs.coverages[*i])},
                                None => {
                                    Some(ecs.coverages[*i])
                                }
                            }
                    }
                }

                // If first entry in stoit, make room
                if stoit_by_entry_by_coverage.len() <= ecs.stoit_index {
                    stoit_by_entry_by_coverage.push(vec![]);
                }
                stoit_by_entry_by_coverage[ecs.stoit_index].push(ecs);
            }

            // Print out coverages iterating over entry IDs.
            for my_entry_i in 0..(stoit_by_entry_by_coverage[0].len()) {
                write!(print_stream, "{}",
                       entry_names[stoit_by_entry_by_coverage[0][0].entry_index]
                       .as_ref().unwrap())
                    .unwrap();
                for stoit_entries in stoit_by_entry_by_coverage.iter() {
                    let ecs = &stoit_entries[my_entry_i as usize];
                    let coverages = &ecs.coverages;
                    for (i, cov) in coverages.iter().enumerate() {
                        if columns_to_normalise.contains(&i) {
                            write!(
                                print_stream, "\t{}",
                                coverages[i]
                                    *100.0
                                    *coverage_multipliers[i]
                                    /coverage_totals[ecs.stoit_index as usize][i].unwrap()).unwrap();
                        } else {
                            write!(print_stream, "\t{}", cov).unwrap();
                        }
                    }
                    writeln!(print_stream).unwrap();
                }
            }
        },
        _ => panic!("programming error")
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::str;

    #[test]
    fn test_dense_cached_printer_hello_world(){
        let mut c = CoverageTakerType::new_cached_single_float_coverage_taker(2);
        c.start_stoit("stoit1");
        c.start_entry(0, "contig1");
        c.add_single_coverage(1.1);
        c.add_single_coverage(1.2);
        let mut stream = Cursor::new(Vec::new());
        print_dense_cached_coverage_taker(
            &"Contig",
            &vec!("mean".to_string(),"std".to_string()),
            &c,
            &mut stream,
            None,
            &vec!());
        assert_eq!("Contig\tstoit1 mean\tstoit1 std\n\
                    contig1\t1.1\t1.2\n",
                   str::from_utf8(stream.get_ref()).unwrap());
    }

    #[test]
    fn test_dense_cached_printer_easy_normalised(){
        let mut c = CoverageTakerType::new_cached_single_float_coverage_taker(2);
        c.start_stoit("stoit1");
        c.start_entry(0, "contig1");
        c.add_single_coverage(1.1);
        c.add_single_coverage(1.2);
        let mut stream = Cursor::new(Vec::new());
        print_dense_cached_coverage_taker(
            &"Contig",
            &vec!("mean".to_string(),"std".to_string()),
            &c,
            &mut stream,
            Some(&vec!(ReadsMapped {
                num_mapped_reads: 1,
                num_reads: 2
            })),
            &vec!(0));
        assert_eq!("Contig\tstoit1 mean\tstoit1 std\n\
                    unmapped\t50\tNA\n\
                    contig1\t50\t1.2\n",
                   str::from_utf8(stream.get_ref()).unwrap());
    }
}
