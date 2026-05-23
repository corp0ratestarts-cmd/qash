(* rocq-qash harness: CLI wrapper for extracted Coq model *)
open Model_extracted

(* Coq extracts Z to a module usually called Z in Model_extracted or similar.
   If we are linking with Zarith, we need to make sure we use the right one. *)

let read_int_list () =
  let line = read_line () in
  let tokens = String.split_on_char ' ' line in
  List.map int_of_string (List.filter (fun s -> s <> "") tokens)

let rec parse_updates n =
  if n <= 0 then []
  else
    let line = read_line () in
    let update =
      if line = "idle" then VU_Idle
      else
        match String.split_on_char ' ' line with
        | [d; c; s] -> VU_Update (int_of_string d, int_of_string c, int_of_string s)
        | _ -> VU_Idle
    in
    update :: parse_updates (n - 1)

let parse_halt_reason = function
  | 0 -> HR_None
  | 1 -> HR_LyapunovViolation
  | 4 -> HR_DecodeInvalid
  | _ -> HR_DecodeInvalid

let serialize_halt_reason = function
  | HR_None -> 0
  | HR_LyapunovViolation -> 1
  | HR_DecodeInvalid -> 4

let () =
  try
    while true do
      (* 1. Read state header: epoch halt_reason validator_count *)
      let header = read_int_list () in
      match header with
      | [epoch; halt_val; vc] ->
          let halt = parse_halt_reason halt_val in
          
          (* 2. Read validator metrics *)
          let rec read_vms n =
            if n <= 0 then []
            else
              match read_int_list () with
              | [d; c; s] -> { vm_D = d; vm_C = c; vm_S = s } :: read_vms (n - 1)
              | _ -> { vm_D = 0; vm_C = 0; vm_S = 0 } :: read_vms (n - 1)
          in
          let vs = read_vms vc in
          
          (* 3. Read window entries: count val1 val2 ... *)
          let window_data = read_int_list () in
          let window = match window_data with
            | _count :: vals -> vals
            | _ -> []
          in
          
          let state = {
            ms_epoch = epoch;
            ms_halt = halt;
            ms_validators = vs;
            ms_window = window;
          } in
          
          (* 4. Read updates *)
          let us = parse_updates vc in
          
          (* 5. Advance epoch *)
          let obs = advance_epoch_observation state us in
          
          (* 6. Output observation: epoch halted(0/1) v_conv delta_window *)
          Printf.printf "%d %d %d %d\n%!"
            obs.obs_epoch
            (if is_halted obs.obs_halt_reason then 1 else 0)
            obs.obs_v_convergence
            obs.obs_delta_window
      | _ -> ()
    done
  with End_of_file -> ()
