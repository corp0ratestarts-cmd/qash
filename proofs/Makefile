COQC ?= coqc
COQDEP ?= coqdep

STRICT_PROOFS := \
	crypto_game_framework.v \
	util/list_inj.v \
	contractivity/lyapunov_stability.v \
	concat_injective.v \
	contractivity/encode_injectivity.v \
	contractivity/tx_perturbation_0.v \
	contractivity/tx1_score_decrement.v \
	contractivity/lyapunov_grace_convergence.v \
	lyapunov_decrease.v \
	safety/absorbing_halt.v \
	integration/th8_composition.v \
	cascade/cascade_health_bounded.v \
	cascade/cascade_determinism.v \
	cascade/cascade_collision_resistance.v \
	cascade/it_mac_forgery_bound.v \
	blinding/blinding_non_interference.v \
	ordering/causal_ordering.v \
	ordering/compatibility_window.v \
	safety/causal_fingerprint.v \
	composition/lyapunov_confluence.v \
	composition/th3_system_closure.v \
	model/Model.v \
	model/RefinementStatement.v \
	model/Extract.v \
	sharding/efb_determinism.v

VFILES := $(STRICT_PROOFS)
VOFILES := $(VFILES:.v=.vo)

all: no-admitted $(VOFILES)

.depend: $(VFILES)
	$(COQDEP) -Q . QASH $(VFILES) > .depend

-include .depend

%.vo: %.v
	$(COQC) -Q . QASH $<

model/Extract.vo: model/Extract.v model/Model.vo
	$(COQC) -Q . QASH $< > /tmp/qash-model-extracted.ml

no-admitted:
	@if grep -R --include='*.v' -n 'Admitted\.' .; then \
		echo 'Admitted proofs are forbidden in checked proof files' >&2; \
		exit 1; \
	fi

clean:
	rm -f $(VOFILES) $(VFILES:.v=.vos) $(VFILES:.v=.vok) $(VFILES:.v=.glob) .depend

.PHONY: all clean no-admitted
